//! Main application state and run loop.

use std::collections::{HashMap, HashSet, VecDeque};
use std::io;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use ratatui::widgets::TableState;
use ratatui::DefaultTerminal;
use strum::{Display, EnumIter, FromRepr};

use crate::audit::{AuditEntry, AuditLogger};
use crate::cleaners::{CleanResult, CleanableItem, ScanResult};
use crate::config::Config;
use crate::db::{Database, DiscoveredPath};
use crate::scanner::InstalledApp;
use crate::trash::TrashMover;

use super::event::{parse_key, AppEvent, EventHandler, KeyAction};
use super::ui;

/// Application running mode.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    #[default]
    Running,
    Confirming(ConfirmAction),
    Scanning,
    Cleaning,
    ViewingCleanReport,
    ShowingHelp,
    Quitting,
}

/// Actions that require user confirmation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmAction {
    CleanSelected,
    CleanAll,
    ClearAuditLog,
    ResetSettings,
}

/// Available tabs in the application.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Display, EnumIter, FromRepr)]
pub enum Tab {
    #[default]
    #[strum(serialize = "Home")]
    Home,
    #[strum(serialize = "Settings")]
    Settings,
}

impl Tab {
    pub fn next(self) -> Self {
        match self {
            Tab::Home => Tab::Settings,
            Tab::Settings => Tab::Home,
        }
    }

    pub fn prev(self) -> Self {
        self.next() // Only 2 tabs, so prev == next
    }

    pub fn from_index(idx: usize) -> Option<Self> {
        Self::from_repr(idx)
    }

    pub fn id(&self) -> &'static str {
        match self {
            Tab::Home => "home",
            Tab::Settings => "settings",
        }
    }
}

/// Status message severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusLevel {
    Info,
    Success,
    Warning,
    Error,
}

/// Dashboard tab state.
#[derive(Default)]
pub struct DashboardState {
    pub total_disk_space: u64,
    pub used_disk_space: u64,
    pub free_disk_space: u64,
    pub category_sizes: Vec<(String, u64)>,
    pub total_reclaimable: u64,
    pub last_scan_time: Option<chrono::DateTime<chrono::Utc>>,
    pub loading: bool,
    pub discovering: bool,
    pub discovery_progress: Option<String>,
    pub discovered_paths_count: usize,
}

/// Scan tab state.
#[derive(Default)]
pub struct ScanTabState {
    pub results: Vec<ScanResult>,
    pub table_state: TableState,
    pub expanded_categories: HashSet<usize>,
    pub scanning: bool,
    pub progress_message: Option<String>,
    pub progress_current: usize,
    pub progress_total: usize,
}

/// Result state for a cleanable row after the last clean operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowCleanStatus {
    Pending,
    Cleaned,
    Failed,
    Skipped,
}

/// Risk filter mode for cleanable list rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RiskFilterMode {
    #[default]
    All,
    Low,
    Medium,
    High,
}

/// Report entry for a failed clean operation.
#[derive(Debug, Clone)]
pub struct CleanFailure {
    pub category: String,
    pub path: PathBuf,
    pub reason: String,
}

/// Persistent report shown after cleaning.
#[derive(Debug, Clone)]
pub struct CleanReportState {
    pub cleaned_count: usize,
    pub failed_count: usize,
    pub skipped_count: usize,
    pub bytes_freed: u64,
    pub top_failure_reasons: Vec<(String, usize)>,
    pub failures: Vec<CleanFailure>,
    pub completed_at: chrono::DateTime<chrono::Utc>,
}

/// An item that can be selected for cleaning.
pub struct SelectableItem {
    pub item: CleanableItem,
    pub category: String,
    pub selected: bool,
    pub clean_status: Option<RowCleanStatus>,
    pub clean_reason: Option<String>,
}

/// A selected item queued for background cleaning.
pub struct QueuedCleanItem {
    pub category: String,
    pub item: CleanableItem,
}

/// Clean tab state.
#[derive(Default)]
pub struct CleanTabState {
    pub items: Vec<SelectableItem>,
    pub table_state: TableState,
    pub selected_count: usize,
    pub selected_size: u64,
    pub selected_low_risk: usize,
    pub selected_medium_risk: usize,
    pub selected_high_risk: usize,
    pub hide_zero_byte_items: bool,
    pub risk_filter_mode: RiskFilterMode,
    pub cleaning: bool,
    pub clean_results: Option<Vec<CleanResult>>,
    pub clean_report: Option<CleanReportState>,
}

/// Disk health tab state.
#[derive(Default)]
pub struct DiskHealthState {
    pub disks: Vec<crate::disk::DiskHealth>,
    pub selected_disk: usize,
    pub table_state: TableState,
    pub smart_available: bool,
    pub loading: bool,
    pub error: Option<String>,
}

/// Audit log tab state.
#[derive(Default)]
pub struct AuditLogState {
    pub entries: Vec<AuditEntry>,
    pub table_state: TableState,
    pub filter_category: Option<String>,
    pub filter_search: Option<String>,
    pub total_entries: usize,
    pub loading: bool,
}

/// A single setting field.
pub struct SettingField {
    pub name: String,
    pub key: String,
    pub description: String,
    pub value: SettingValue,
    pub category: String,
}

/// Setting value types.
#[derive(Clone)]
pub enum SettingValue {
    Bool(bool),
    Number(u64),
    String(String),
}

/// Settings tab state.
#[derive(Default)]
pub struct SettingsState {
    pub selected_field: usize,
    pub fields: Vec<SettingField>,
    pub editing: bool,
    pub edit_buffer: String,
    pub unsaved_changes: bool,
}

/// Which panel is focused in two-column view.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum FocusPanel {
    #[default]
    Left, // Cleanable items
    Right, // Apps
}

/// Apps panel state.
#[derive(Default)]
pub struct AppsState {
    pub apps: Vec<InstalledApp>,
    pub table_state: TableState,
    pub selected_for_uninstall: HashSet<usize>,
    pub scanning: bool,
}

/// Background task types.
pub enum BackgroundTask {
    ScanCategories,
    ScanApps,
    CleanItems(Vec<QueuedCleanItem>),
    UninstallApps(Vec<InstalledApp>),
    RefreshDiskHealth,
    LoadAuditLog,
    DiscoverPaths,
}

/// Task results from background operations.
pub enum TaskResult {
    ScanComplete(Vec<ScanResult>),
    ScanProgress(String, usize, usize),
    AppsScanned(Vec<InstalledApp>),
    CleanComplete(Vec<CleanResult>),
    UninstallComplete(usize, u64), // (count, bytes freed)
    DiskHealthLoaded(Vec<crate::disk::DiskHealth>),
    AuditLogLoaded(Vec<AuditEntry>),
    DiscoveryProgress(String),
    DiscoveryComplete(Vec<DiscoveredPath>),
    Error(String),
}

/// Main application state.
pub struct App {
    // Core state
    pub mode: AppMode,
    pub current_tab: Tab,
    pub config: Config,
    pub focus_panel: FocusPanel,

    // Shared resources
    pub trash_mover: Option<TrashMover>,
    pub audit_logger: AuditLogger,
    pub database: Option<Database>,

    // Tab-specific state
    pub dashboard: DashboardState,
    pub scan: ScanTabState,
    pub clean: CleanTabState,
    pub apps: AppsState,
    pub disk_health: DiskHealthState,
    pub audit_log: AuditLogState,
    pub settings: SettingsState,

    // Background task communication
    pub task_tx: Option<Sender<BackgroundTask>>,
    pub result_rx: Option<Receiver<TaskResult>>,

    // Status message
    pub status_message: Option<(String, StatusLevel, std::time::Instant)>,

    // Event handler
    event_handler: EventHandler,
}

impl App {
    /// Build cleaner instances from config, honoring category enable flags.
    fn build_cleaners(
        config: &Config,
        audit_enabled: bool,
    ) -> Vec<Box<dyn crate::cleaners::Cleaner>> {
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        use crate::cleaners::HomebrewCleaner;
        #[cfg(target_os = "macos")]
        use crate::cleaners::XcodeCleaner;
        use crate::cleaners::{
            AppsLeftoversCleaner, CargoCleaner, Cleaner, DockerCleaner, NpmCleaner, PipCleaner,
            SystemCachesCleaner, SystemLogsCleaner, TrashCleaner, YarnCleaner,
        };

        let mut cleaners: Vec<Box<dyn Cleaner>> = Vec::new();

        if config.categories.system_cache.enabled {
            if let Ok(tm) = TrashMover::default_mover() {
                cleaners.push(Box::new(SystemCachesCleaner::new(
                    tm,
                    AuditLogger::default_logger(audit_enabled),
                    &config.categories.system_cache,
                )));
            }
        }

        if config.categories.system_logs.enabled {
            if let Ok(tm) = TrashMover::default_mover() {
                cleaners.push(Box::new(SystemLogsCleaner::new(
                    tm,
                    AuditLogger::default_logger(audit_enabled),
                    &config.categories.system_logs,
                )));
            }
        }

        #[cfg(target_os = "macos")]
        if config.categories.xcode.base.enabled {
            if let Ok(tm) = TrashMover::default_mover() {
                cleaners.push(Box::new(XcodeCleaner::new(
                    tm,
                    AuditLogger::default_logger(audit_enabled),
                    &config.categories.xcode,
                )));
            }
        }

        if config.categories.npm.enabled {
            if let Ok(tm) = TrashMover::default_mover() {
                cleaners.push(Box::new(NpmCleaner::new(
                    tm,
                    AuditLogger::default_logger(audit_enabled),
                    &config.categories.npm,
                )));
            }
        }

        if config.categories.yarn.enabled {
            if let Ok(tm) = TrashMover::default_mover() {
                cleaners.push(Box::new(YarnCleaner::new(
                    tm,
                    AuditLogger::default_logger(audit_enabled),
                    &config.categories.yarn,
                )));
            }
        }

        if config.categories.cargo.base.enabled {
            if let Ok(tm) = TrashMover::default_mover() {
                cleaners.push(Box::new(CargoCleaner::new(
                    tm,
                    AuditLogger::default_logger(audit_enabled),
                    &config.categories.cargo,
                )));
            }
        }

        if config.categories.pip.enabled {
            if let Ok(tm) = TrashMover::default_mover() {
                cleaners.push(Box::new(PipCleaner::new(
                    tm,
                    AuditLogger::default_logger(audit_enabled),
                    &config.categories.pip,
                )));
            }
        }

        if config.categories.trash.base.enabled {
            cleaners.push(Box::new(TrashCleaner::new(
                AuditLogger::default_logger(audit_enabled),
                &config.categories.trash,
            )));
        }

        #[cfg(any(target_os = "macos", target_os = "linux"))]
        if config.categories.homebrew.base.enabled {
            cleaners.push(Box::new(HomebrewCleaner::new(
                AuditLogger::default_logger(audit_enabled),
                &config.categories.homebrew,
            )));
        }

        if config.categories.docker.base.enabled {
            cleaners.push(Box::new(DockerCleaner::new(
                AuditLogger::default_logger(audit_enabled),
                &config.categories.docker,
            )));
        }

        if config.categories.apps.base.enabled {
            cleaners.push(Box::new(AppsLeftoversCleaner::new(
                AuditLogger::default_logger(audit_enabled),
                &config.categories.apps,
            )));
        }

        cleaners
    }

    /// Create a new application instance.
    pub fn new(config: Config) -> Self {
        let trash_mover = TrashMover::default_mover().ok();
        let audit_logger = AuditLogger::default_logger(config.safety.audit_enabled);
        let database = Database::open().ok();

        if database.is_none() {
            tracing::warn!("Could not open database, some features may be limited");
        }

        // Load discovered paths count from database
        let discovered_paths_count = database
            .as_ref()
            .and_then(|db| db.get_active_discovered_paths().ok())
            .map(|paths| paths.len())
            .unwrap_or(0);

        // Initialize settings fields from config
        let settings = SettingsState {
            fields: Self::config_to_fields(&config),
            ..Default::default()
        };
        let clean = CleanTabState {
            hide_zero_byte_items: config.tui.hide_zero_byte_items,
            ..Default::default()
        };

        Self {
            mode: AppMode::Running,
            current_tab: Tab::Home,
            config,
            focus_panel: FocusPanel::Left,
            trash_mover,
            audit_logger,
            database,
            dashboard: DashboardState {
                discovered_paths_count,
                ..Default::default()
            },
            scan: ScanTabState::default(),
            clean,
            apps: AppsState::default(),
            disk_health: DiskHealthState::default(),
            audit_log: AuditLogState::default(),
            settings,
            task_tx: None,
            result_rx: None,
            status_message: None,
            event_handler: EventHandler::default(),
        }
    }

    /// Convert config to editable fields.
    fn config_to_fields(config: &Config) -> Vec<SettingField> {
        vec![
            SettingField {
                name: "Keep Recent Days".to_string(),
                key: "general.keep_recent_days".to_string(),
                description: "Number of days to keep recent files".to_string(),
                value: SettingValue::Number(config.general.keep_recent_days as u64),
                category: "General".to_string(),
            },
            SettingField {
                name: "Warn Threshold".to_string(),
                key: "safety.warn_threshold".to_string(),
                description: "Size threshold for warning (bytes)".to_string(),
                value: SettingValue::Number(config.safety.warn_threshold),
                category: "Safety".to_string(),
            },
            SettingField {
                name: "Force Threshold".to_string(),
                key: "safety.force_threshold".to_string(),
                description: "Size threshold requiring confirmation (bytes)".to_string(),
                value: SettingValue::Number(config.safety.force_threshold),
                category: "Safety".to_string(),
            },
            SettingField {
                name: "Always Confirm".to_string(),
                key: "safety.always_confirm".to_string(),
                description: "Always require confirmation before cleaning".to_string(),
                value: SettingValue::Bool(config.safety.always_confirm),
                category: "Safety".to_string(),
            },
            SettingField {
                name: "Audit Logging".to_string(),
                key: "safety.audit_enabled".to_string(),
                description: "Enable audit logging".to_string(),
                value: SettingValue::Bool(config.safety.audit_enabled),
                category: "Safety".to_string(),
            },
        ]
    }

    /// Run the application.
    pub fn run(mut self, mut terminal: DefaultTerminal) -> io::Result<()> {
        // Spawn background worker thread
        self.spawn_background_worker();

        // Initial data load
        self.refresh_dashboard();
        self.load_disk_health();

        // Auto-start scan on launch
        self.start_scan();
        self.start_app_scan();

        // Main loop
        while self.mode != AppMode::Quitting {
            // Draw UI
            terminal.draw(|frame| ui::render(&mut self, frame))?;

            // Poll for background task results
            self.poll_results();

            // Handle events
            match self.event_handler.next()? {
                AppEvent::Key(key) => {
                    let action = parse_key(key);
                    self.handle_action(action);
                }
                AppEvent::Resize(_, _) => {
                    // Terminal will redraw automatically
                }
                AppEvent::Tick => {
                    // Clear old status messages
                    self.clear_old_status();
                }
            }
        }

        Ok(())
    }

    /// Spawn the background worker thread.
    fn spawn_background_worker(&mut self) {
        let (task_tx, task_rx) = mpsc::channel::<BackgroundTask>();
        let (result_tx, result_rx) = mpsc::channel::<TaskResult>();

        self.task_tx = Some(task_tx);
        self.result_rx = Some(result_rx);

        let fallback_config = self.config.clone();

        thread::spawn(move || {
            while let Ok(task) = task_rx.recv() {
                match task {
                    BackgroundTask::ScanCategories => {
                        let config = crate::config::load_config(None)
                            .unwrap_or_else(|_| fallback_config.clone());
                        let results = Self::do_scan(&config, &result_tx);
                        let _ = result_tx.send(TaskResult::ScanComplete(results));
                    }
                    BackgroundTask::CleanItems(items) => {
                        let config = crate::config::load_config(None)
                            .unwrap_or_else(|_| fallback_config.clone());
                        let results = Self::do_clean(&items, &config);
                        let _ = result_tx.send(TaskResult::CleanComplete(results));
                    }
                    BackgroundTask::RefreshDiskHealth => {
                        let disks = crate::disk::get_all_disk_health();
                        let _ = result_tx.send(TaskResult::DiskHealthLoaded(disks));
                    }
                    BackgroundTask::LoadAuditLog => {
                        let logger = AuditLogger::default_logger(true);
                        if let Ok(entries) = logger.read_recent(1000) {
                            let _ = result_tx.send(TaskResult::AuditLogLoaded(entries));
                        }
                    }
                    BackgroundTask::DiscoverPaths => {
                        Self::do_discover_paths(&result_tx);
                    }
                    BackgroundTask::ScanApps => {
                        let apps = crate::scanner::scan_applications();
                        let _ = result_tx.send(TaskResult::AppsScanned(apps));
                    }
                    BackgroundTask::UninstallApps(apps) => {
                        let (count, size) = Self::do_uninstall_apps(&apps);
                        let _ = result_tx.send(TaskResult::UninstallComplete(count, size));
                    }
                }
            }
        });
    }

    /// Perform path discovery in background.
    fn do_discover_paths(tx: &Sender<TaskResult>) {
        use crate::discovery::{DiscoveryConfig, PathDiscovery};

        let db = match Database::open() {
            Ok(db) => db,
            Err(e) => {
                let _ = tx.send(TaskResult::Error(format!("Database error: {}", e)));
                return;
            }
        };

        let discovery = PathDiscovery::new(db, DiscoveryConfig::default());

        // Use a callback to report progress
        let progress_callback = |msg: &str| {
            let _ = tx.send(TaskResult::DiscoveryProgress(msg.to_string()));
        };

        match discovery.discover_all(Some(&progress_callback)) {
            Ok(paths) => {
                let _ = tx.send(TaskResult::DiscoveryComplete(paths));
            }
            Err(e) => {
                let _ = tx.send(TaskResult::Error(format!("Discovery failed: {}", e)));
            }
        }
    }

    fn resolve_scan_parallelism(config: &Config, cleaner_count: usize) -> usize {
        const MAX_PARALLELISM: usize = 8;

        if cleaner_count == 0 {
            return 1;
        }

        let requested = config.tui.scan_parallelism as usize;
        let auto = thread::available_parallelism()
            .map(|p| p.get())
            .unwrap_or(4);
        let workers = if requested == 0 { auto } else { requested };

        workers.clamp(1, cleaner_count.min(MAX_PARALLELISM))
    }

    /// Perform scanning in background.
    fn do_scan(config: &Config, tx: &Sender<TaskResult>) -> Vec<ScanResult> {
        use crate::cleaners::CleanerContext;
        use crate::discovery::{DiscoveryConfig, PathDiscovery};

        enum WorkerUpdate {
            Started(String),
            Finished {
                index: usize,
                cleaner_name: String,
                result: Option<ScanResult>,
            },
        }

        // First, run discovery to find new paths (silently)
        let _ = tx.send(TaskResult::ScanProgress(
            "Discovering paths...".to_string(),
            0,
            1,
        ));
        if let Ok(db) = Database::open() {
            let discovery = PathDiscovery::new(db, DiscoveryConfig::default());
            // Run incremental discovery (faster, uses cache)
            let _ = discovery.discover_incremental();
        }

        // Disable audit logging for scans.
        let cleaners = Self::build_cleaners(config, false);
        let total = cleaners.len();
        if total == 0 {
            return Vec::new();
        }

        let ctx = CleanerContext {
            dry_run: true,
            keep_recent_days: config.general.keep_recent_days,
            ..Default::default()
        };
        let parallelism = Self::resolve_scan_parallelism(config, total);
        let _ = tx.send(TaskResult::ScanProgress(
            format!("Scanning with {} worker(s)...", parallelism),
            0,
            total,
        ));

        let queue = Arc::new(Mutex::new(
            cleaners
                .into_iter()
                .enumerate()
                .collect::<VecDeque<(usize, Box<dyn crate::cleaners::Cleaner>)>>(),
        ));
        let (update_tx, update_rx) = mpsc::channel::<WorkerUpdate>();
        let mut handles = Vec::with_capacity(parallelism);

        for _ in 0..parallelism {
            let queue = Arc::clone(&queue);
            let update_tx = update_tx.clone();
            let worker_ctx = ctx.clone();

            handles.push(thread::spawn(move || loop {
                let task = match queue.lock() {
                    Ok(mut guard) => guard.pop_front(),
                    Err(_) => None,
                };

                let Some((index, cleaner)) = task else {
                    break;
                };

                let cleaner_name = cleaner.name().to_string();
                let _ = update_tx.send(WorkerUpdate::Started(cleaner_name.clone()));
                let result = if cleaner.is_available() {
                    Some(cleaner.scan(&worker_ctx))
                } else {
                    None
                };
                let _ = update_tx.send(WorkerUpdate::Finished {
                    index,
                    cleaner_name,
                    result,
                });
            }));
        }

        drop(update_tx);

        let mut completed = 0usize;
        let mut active_cleaners: HashSet<String> = HashSet::new();
        let mut results_by_index: Vec<Option<ScanResult>> = vec![None; total];

        while completed < total {
            match update_rx.recv() {
                Ok(WorkerUpdate::Started(cleaner_name)) => {
                    active_cleaners.insert(cleaner_name.clone());
                    let _ = tx.send(TaskResult::ScanProgress(
                        format!("Scanning {} ({}/{})", cleaner_name, completed, total),
                        completed,
                        total,
                    ));
                }
                Ok(WorkerUpdate::Finished {
                    index,
                    cleaner_name,
                    result,
                }) => {
                    completed += 1;
                    active_cleaners.remove(&cleaner_name);
                    if let Some(scan_result) = result {
                        results_by_index[index] = Some(scan_result);
                    }

                    let progress_msg = if let Some(active) = active_cleaners.iter().next() {
                        format!("Scanning {} ({}/{})", active, completed, total)
                    } else {
                        format!("Scan progress ({}/{})", completed, total)
                    };
                    let _ = tx.send(TaskResult::ScanProgress(progress_msg, completed, total));
                }
                Err(_) => break,
            }
        }

        for handle in handles {
            let _ = handle.join();
        }

        results_by_index.into_iter().flatten().collect()
    }

    /// Perform cleaning in background.
    fn do_clean(items: &[QueuedCleanItem], config: &Config) -> Vec<CleanResult> {
        use crate::cleaners::CleanerContext;
        use std::time::Duration;

        if items.is_empty() {
            return Vec::new();
        }

        let cleaners = Self::build_cleaners(config, config.safety.audit_enabled);
        let mut by_category: HashMap<String, Vec<CleanableItem>> = HashMap::new();

        for queued in items {
            by_category
                .entry(queued.category.clone())
                .or_default()
                .push(queued.item.clone());
        }

        let ctx = CleanerContext {
            dry_run: false,
            force: false,
            keep_recent_days: config.general.keep_recent_days,
            min_size: None,
            max_items: None,
            skip_confirm: true,
        };

        let mut results = Vec::new();

        for cleaner in cleaners {
            if let Some(category_items) = by_category.remove(cleaner.id()) {
                if !category_items.is_empty() {
                    results.push(cleaner.clean(&category_items, &ctx));
                }
            }
        }

        // If a category has no matching cleaner, return explicit failures.
        for (category, category_items) in by_category {
            let failed = category_items
                .into_iter()
                .map(|item| {
                    (
                        item.path,
                        format!("No cleaner available for category '{}'", category),
                    )
                })
                .collect();

            results.push(CleanResult {
                category,
                items_cleaned: 0,
                bytes_freed: 0,
                items_failed: failed,
                clean_duration: Duration::ZERO,
            });
        }

        results
    }

    /// Uninstall apps and clean their leftovers.
    fn do_uninstall_apps(apps: &[InstalledApp]) -> (usize, u64) {
        let mut count = 0;
        let mut total_size = 0u64;

        let trash_mover = match TrashMover::default_mover() {
            Ok(tm) => tm,
            Err(_) => return (0, 0),
        };

        for app in apps {
            // Move app bundle to trash
            if trash_mover.move_to_trash(&app.path).is_ok() {
                count += 1;
                total_size += app.size;

                // Clean leftovers (use pre-calculated leftover_size)
                for leftover in &app.leftovers {
                    if leftover.exists() {
                        let _ = trash_mover.move_to_trash(leftover);
                    }
                }
                total_size += app.leftover_size;
            }
        }

        (count, total_size)
    }

    /// Poll for background task results.
    fn poll_results(&mut self) {
        // Collect results first to avoid borrow issues
        let results: Vec<TaskResult> = if let Some(rx) = &self.result_rx {
            let mut collected = Vec::new();
            while let Ok(result) = rx.try_recv() {
                collected.push(result);
            }
            collected
        } else {
            Vec::new()
        };

        // Now process results
        for result in results {
            match result {
                TaskResult::ScanComplete(scan_results) => {
                    self.scan.scanning = false;
                    self.scan.progress_message = None;
                    if self.mode == AppMode::Scanning {
                        self.mode = AppMode::Running;
                    }
                    self.update_dashboard_from_scan(&scan_results);
                    self.populate_clean_items(&scan_results);
                    self.scan.results = scan_results;
                    self.set_status("Scan complete", StatusLevel::Success);
                }
                TaskResult::ScanProgress(msg, current, total) => {
                    self.scan.progress_message = Some(msg);
                    self.scan.progress_current = current;
                    self.scan.progress_total = total;
                }
                TaskResult::CleanComplete(clean_results) => {
                    self.clean.cleaning = false;
                    let skipped_count = self.apply_clean_results_to_rows(&clean_results);
                    let report = Self::build_clean_report(&clean_results, skipped_count);
                    self.clean.clean_results = Some(clean_results);
                    self.clean.clean_report = Some(report.clone());
                    self.update_selection_stats();
                    self.mode = if self.config.tui.show_clean_report_on_complete {
                        AppMode::ViewingCleanReport
                    } else {
                        AppMode::Running
                    };
                    self.set_status(
                        &format!(
                            "Cleaned {} | Failed {} | Skipped {} | Freed {}",
                            report.cleaned_count,
                            report.failed_count,
                            report.skipped_count,
                            crate::scanner::format_size(report.bytes_freed)
                        ),
                        if report.failed_count > 0 {
                            StatusLevel::Warning
                        } else {
                            StatusLevel::Success
                        },
                    );
                    // Refresh disk info
                    self.refresh_dashboard();
                }
                TaskResult::DiskHealthLoaded(disks) => {
                    self.disk_health.loading = false;
                    self.disk_health.smart_available = disks.iter().any(|d| d.smart_data.is_some());
                    self.disk_health.disks = disks;
                }
                TaskResult::AuditLogLoaded(entries) => {
                    self.audit_log.loading = false;
                    self.audit_log.total_entries = entries.len();
                    self.audit_log.entries = entries;
                }
                TaskResult::DiscoveryProgress(msg) => {
                    self.dashboard.discovery_progress = Some(msg);
                }
                TaskResult::DiscoveryComplete(paths) => {
                    self.dashboard.discovering = false;
                    self.dashboard.discovery_progress = None;
                    self.dashboard.discovered_paths_count = paths.len();
                    self.set_status(
                        &format!("Discovery complete: {} paths found", paths.len()),
                        StatusLevel::Success,
                    );
                }
                TaskResult::AppsScanned(apps) => {
                    self.apps.scanning = false;
                    self.apps.apps = apps;
                    if self.apps.apps.is_empty() {
                        self.set_status("No apps found", StatusLevel::Info);
                    }
                }
                TaskResult::UninstallComplete(count, size) => {
                    self.apps.selected_for_uninstall.clear();
                    // Remove uninstalled apps from list
                    self.apps.apps.retain(|app| app.path.exists());
                    self.set_status(
                        &format!(
                            "Uninstalled {} apps ({})",
                            count,
                            crate::scanner::format_size(size)
                        ),
                        StatusLevel::Success,
                    );
                    self.refresh_dashboard();
                }
                TaskResult::Error(msg) => {
                    self.dashboard.discovering = false;
                    self.scan.scanning = false;
                    self.clean.cleaning = false;
                    if matches!(self.mode, AppMode::Cleaning | AppMode::Scanning) {
                        self.mode = AppMode::Running;
                    }
                    for item in &mut self.clean.items {
                        if item.clean_status == Some(RowCleanStatus::Pending) {
                            item.clean_status = Some(RowCleanStatus::Failed);
                            item.clean_reason = Some(msg.clone());
                            item.selected = false;
                        }
                    }
                    self.update_selection_stats();
                    self.set_status(&msg, StatusLevel::Error);
                }
            }
        }
    }

    /// Update dashboard from scan results.
    fn update_dashboard_from_scan(&mut self, results: &[ScanResult]) {
        self.dashboard.category_sizes = results
            .iter()
            .map(|r| (r.category.clone(), r.total_size))
            .collect();
        self.dashboard.total_reclaimable = results.iter().map(|r| r.total_size).sum();
        self.dashboard.last_scan_time = Some(chrono::Utc::now());
    }

    /// Populate clean items from scan results.
    fn populate_clean_items(&mut self, results: &[ScanResult]) {
        self.clean.items.clear();
        for result in results {
            for item in &result.items {
                self.clean.items.push(SelectableItem {
                    item: item.clone(),
                    category: result.category.clone(),
                    selected: false,
                    clean_status: None,
                    clean_reason: None,
                });
            }
        }

        let visible = self.visible_clean_indices();
        if visible.is_empty() {
            self.clean.table_state.select(None);
        } else {
            self.clean.table_state.select(Some(0));
        }
        self.update_selection_stats();
    }

    /// Handle a user action.
    fn handle_action(&mut self, action: KeyAction) {
        // Handle mode-specific actions first
        match self.mode {
            AppMode::Confirming(confirm_action) => {
                match action {
                    KeyAction::Confirm => {
                        self.mode = AppMode::Running;
                        self.execute_confirmed_action(confirm_action);
                    }
                    KeyAction::ForceQuit => {
                        self.mode = AppMode::Quitting;
                    }
                    KeyAction::Cancel | KeyAction::Quit | KeyAction::SelectNone => {
                        self.mode = AppMode::Running;
                    }
                    _ => {}
                }
                return;
            }
            AppMode::ViewingCleanReport => {
                match action {
                    KeyAction::Cancel | KeyAction::ViewReport => {
                        self.mode = AppMode::Running;
                    }
                    KeyAction::Quit | KeyAction::ForceQuit => {
                        self.mode = AppMode::Quitting;
                    }
                    _ => {}
                }
                return;
            }
            AppMode::ShowingHelp => {
                // Any key closes help
                self.mode = AppMode::Running;
                return;
            }
            _ => {}
        }

        // Global actions
        match action {
            KeyAction::Quit | KeyAction::ForceQuit => {
                self.mode = AppMode::Quitting;
                return;
            }
            KeyAction::Help => {
                self.mode = AppMode::ShowingHelp;
                return;
            }
            KeyAction::NextTab => {
                self.current_tab = self.current_tab.next();
                return;
            }
            KeyAction::PrevTab => {
                self.current_tab = self.current_tab.prev();
                return;
            }
            KeyAction::JumpToTab(idx) => {
                if let Some(tab) = Tab::from_index(idx) {
                    self.current_tab = tab;
                }
                return;
            }
            _ => {}
        }

        // Tab-specific actions
        match self.current_tab {
            Tab::Home => self.handle_home_action(action),
            Tab::Settings => self.handle_settings_action(action),
        }
    }

    fn handle_home_action(&mut self, action: KeyAction) {
        match action {
            // Scanning (includes automatic discovery)
            KeyAction::StartScan | KeyAction::Refresh => {
                if !self.scan.scanning && !self.apps.scanning {
                    self.start_scan();
                    self.start_app_scan();
                }
            }
            KeyAction::Discover => {
                if !self.dashboard.discovering {
                    self.dashboard.discovering = true;
                    self.dashboard.discovery_progress = Some("Starting discovery...".to_string());
                    if let Some(tx) = &self.task_tx {
                        let _ = tx.send(BackgroundTask::DiscoverPaths);
                    }
                }
            }
            KeyAction::ToggleZeroByteRows => {
                self.clean.hide_zero_byte_items = !self.clean.hide_zero_byte_items;
                self.config.tui.hide_zero_byte_items = self.clean.hide_zero_byte_items;
                self.ensure_clean_selection_valid();
                self.set_status(
                    if self.clean.hide_zero_byte_items {
                        "Hiding 0 B rows"
                    } else {
                        "Showing 0 B rows"
                    },
                    StatusLevel::Info,
                );
            }
            KeyAction::ViewReport => {
                if self.clean.clean_report.is_some() {
                    self.mode = AppMode::ViewingCleanReport;
                } else {
                    self.set_status("No clean report available yet", StatusLevel::Info);
                }
            }
            // Panel switching (Left/Right or h/l)
            KeyAction::Left | KeyAction::Right => {
                self.focus_panel = match self.focus_panel {
                    FocusPanel::Left => FocusPanel::Right,
                    FocusPanel::Right => FocusPanel::Left,
                };
            }
            // Navigation (within focused panel)
            KeyAction::Down => match self.focus_panel {
                FocusPanel::Left => {
                    let len = self.visible_clean_indices().len();
                    if len > 0 {
                        let i = self.clean.table_state.selected().unwrap_or(0);
                        self.clean.table_state.select(Some((i + 1).min(len - 1)));
                    }
                }
                FocusPanel::Right => {
                    let len = self.apps.apps.len();
                    if len > 0 {
                        let i = self.apps.table_state.selected().unwrap_or(0);
                        self.apps.table_state.select(Some((i + 1).min(len - 1)));
                    }
                }
            },
            KeyAction::Up => match self.focus_panel {
                FocusPanel::Left => {
                    let i = self.clean.table_state.selected().unwrap_or(0);
                    self.clean.table_state.select(Some(i.saturating_sub(1)));
                }
                FocusPanel::Right => {
                    let i = self.apps.table_state.selected().unwrap_or(0);
                    self.apps.table_state.select(Some(i.saturating_sub(1)));
                }
            },
            // Selection (within focused panel)
            KeyAction::Select => match self.focus_panel {
                FocusPanel::Left => {
                    let visible = self.visible_clean_indices();
                    if let Some(visible_idx) = self.clean.table_state.selected() {
                        if let Some(&item_idx) = visible.get(visible_idx) {
                            let item = &mut self.clean.items[item_idx];
                            item.selected = !item.selected;
                            self.update_selection_stats();
                        }
                    }
                }
                FocusPanel::Right => {
                    if let Some(i) = self.apps.table_state.selected() {
                        if i < self.apps.apps.len() {
                            if self.apps.selected_for_uninstall.contains(&i) {
                                self.apps.selected_for_uninstall.remove(&i);
                            } else {
                                self.apps.selected_for_uninstall.insert(i);
                            }
                        }
                    }
                }
            },
            KeyAction::SelectAll => match self.focus_panel {
                FocusPanel::Left => {
                    for idx in self.visible_clean_indices() {
                        if let Some(item) = self.clean.items.get_mut(idx) {
                            item.selected = true;
                        }
                    }
                    self.update_selection_stats();
                }
                FocusPanel::Right => {
                    for i in 0..self.apps.apps.len() {
                        self.apps.selected_for_uninstall.insert(i);
                    }
                }
            },
            KeyAction::SelectNone => match self.focus_panel {
                FocusPanel::Left => {
                    for item in &mut self.clean.items {
                        item.selected = false;
                    }
                    self.update_selection_stats();
                }
                FocusPanel::Right => {
                    self.apps.selected_for_uninstall.clear();
                }
            },
            // Clean/Uninstall action
            KeyAction::StartClean => match self.focus_panel {
                FocusPanel::Left => {
                    if self.clean.selected_count > 0 && !self.clean.cleaning {
                        self.mode = AppMode::Confirming(ConfirmAction::CleanSelected);
                    }
                }
                FocusPanel::Right => {
                    if !self.apps.selected_for_uninstall.is_empty() {
                        self.start_uninstall_apps();
                    }
                }
            },
            _ => {}
        }
    }

    pub fn visible_clean_indices(&self) -> Vec<usize> {
        self.clean
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| !self.clean.hide_zero_byte_items || item.item.size > 0)
            .filter(|(_, item)| match self.clean.risk_filter_mode {
                RiskFilterMode::All => true,
                RiskFilterMode::Low => {
                    matches!(item.item.risk_level, crate::cleaners::RiskLevel::Low)
                }
                RiskFilterMode::Medium => {
                    matches!(item.item.risk_level, crate::cleaners::RiskLevel::Medium)
                }
                RiskFilterMode::High => {
                    matches!(item.item.risk_level, crate::cleaners::RiskLevel::High)
                }
            })
            .map(|(idx, _)| idx)
            .collect()
    }

    fn ensure_clean_selection_valid(&mut self) {
        let visible_len = self.visible_clean_indices().len();
        if visible_len == 0 {
            self.clean.table_state.select(None);
            return;
        }

        let selected = self.clean.table_state.selected().unwrap_or(0);
        self.clean
            .table_state
            .select(Some(selected.min(visible_len.saturating_sub(1))));
    }

    /// Start app uninstall process.
    fn start_uninstall_apps(&mut self) {
        let apps_to_uninstall: Vec<InstalledApp> = self
            .apps
            .selected_for_uninstall
            .iter()
            .filter_map(|&i| self.apps.apps.get(i).cloned())
            .collect();

        if apps_to_uninstall.is_empty() {
            return;
        }

        if let Some(tx) = &self.task_tx {
            if tx
                .send(BackgroundTask::UninstallApps(apps_to_uninstall))
                .is_ok()
            {
                self.set_status("Uninstalling apps...", StatusLevel::Info);
            }
        }
    }

    /// Start scanning for installed apps.
    fn start_app_scan(&mut self) {
        if let Some(tx) = &self.task_tx {
            if tx.send(BackgroundTask::ScanApps).is_ok() {
                self.apps.scanning = true;
            }
        }
    }

    fn handle_settings_action(&mut self, action: KeyAction) {
        match action {
            KeyAction::Down => {
                let len = self.settings.fields.len();
                if len > 0 {
                    self.settings.selected_field = (self.settings.selected_field + 1).min(len - 1);
                }
            }
            KeyAction::Up => {
                self.settings.selected_field = self.settings.selected_field.saturating_sub(1);
            }
            KeyAction::Select => {
                // Toggle boolean fields
                if let Some(field) = self.settings.fields.get_mut(self.settings.selected_field) {
                    if let SettingValue::Bool(ref mut v) = field.value {
                        *v = !*v;
                        self.settings.unsaved_changes = true;
                    }
                }
            }
            KeyAction::Save => {
                self.save_settings();
            }
            KeyAction::Reset => {
                self.mode = AppMode::Confirming(ConfirmAction::ResetSettings);
            }
            _ => {}
        }
    }

    fn execute_confirmed_action(&mut self, action: ConfirmAction) {
        match action {
            ConfirmAction::CleanSelected => {
                self.start_clean();
            }
            ConfirmAction::CleanAll => {
                // Select all and clean
                for item in &mut self.clean.items {
                    item.selected = true;
                }
                self.start_clean();
            }
            ConfirmAction::ClearAuditLog => {
                // Clear by removing entries older than 0 days (all entries)
                let _ = self.audit_logger.cleanup_old_entries(0);
                self.audit_log.entries.clear();
                self.set_status("Audit log cleared", StatusLevel::Success);
            }
            ConfirmAction::ResetSettings => {
                self.settings.fields = Self::config_to_fields(&Config::default());
                self.settings.unsaved_changes = true;
                self.set_status("Settings reset to defaults", StatusLevel::Info);
            }
        }
    }

    /// Start scanning.
    fn start_scan(&mut self) {
        if self.scan.scanning {
            return;
        }

        self.scan.scanning = true;
        if self.mode == AppMode::Running {
            self.mode = AppMode::Scanning;
        }
        self.scan.results.clear();
        self.scan.progress_message = Some("Starting scan...".to_string());

        if let Some(tx) = &self.task_tx {
            let _ = tx.send(BackgroundTask::ScanCategories);
        }
    }

    /// Start cleaning selected items.
    fn start_clean(&mut self) {
        if self.clean.cleaning {
            return;
        }

        let items: Vec<QueuedCleanItem> = self
            .clean
            .items
            .iter()
            .filter(|i| i.selected)
            .map(|i| QueuedCleanItem {
                category: i.category.clone(),
                item: i.item.clone(),
            })
            .collect();

        if items.is_empty() {
            return;
        }

        for item in &mut self.clean.items {
            if item.selected {
                item.clean_status = Some(RowCleanStatus::Pending);
                item.clean_reason = None;
            }
        }

        self.clean.cleaning = true;
        self.mode = AppMode::Cleaning;
        self.set_status("Cleaning selected items...", StatusLevel::Info);

        if let Some(tx) = &self.task_tx {
            let _ = tx.send(BackgroundTask::CleanItems(items));
        }
    }

    /// Update selection statistics.
    fn update_selection_stats(&mut self) {
        self.clean.selected_count = self.clean.items.iter().filter(|i| i.selected).count();
        self.clean.selected_size = self
            .clean
            .items
            .iter()
            .filter(|i| i.selected)
            .map(|i| i.item.size)
            .sum();
        self.clean.selected_low_risk = self
            .clean
            .items
            .iter()
            .filter(|i| i.selected && matches!(i.item.risk_level, crate::cleaners::RiskLevel::Low))
            .count();
        self.clean.selected_medium_risk = self
            .clean
            .items
            .iter()
            .filter(|i| {
                i.selected && matches!(i.item.risk_level, crate::cleaners::RiskLevel::Medium)
            })
            .count();
        self.clean.selected_high_risk = self
            .clean
            .items
            .iter()
            .filter(|i| i.selected && matches!(i.item.risk_level, crate::cleaners::RiskLevel::High))
            .count();
    }

    fn apply_clean_results_to_rows(&mut self, clean_results: &[CleanResult]) -> usize {
        let mut failure_map: HashMap<(String, PathBuf), String> = HashMap::new();
        for result in clean_results {
            for (path, reason) in &result.items_failed {
                failure_map.insert((result.category.clone(), path.clone()), reason.clone());
            }
        }

        let mut skipped_count = 0usize;

        for item in &mut self.clean.items {
            if item.clean_status != Some(RowCleanStatus::Pending) {
                continue;
            }

            let key = (item.category.clone(), item.item.path.clone());
            if let Some(reason) = failure_map.get(&key) {
                item.clean_status = Some(RowCleanStatus::Failed);
                item.clean_reason = Some(reason.clone());
            } else if !item.item.path.exists() {
                item.clean_status = Some(RowCleanStatus::Cleaned);
                item.clean_reason = None;
            } else {
                item.clean_status = Some(RowCleanStatus::Skipped);
                item.clean_reason = Some("Item still exists after clean".to_string());
                skipped_count += 1;
            }

            item.selected = false;
        }

        skipped_count
    }

    fn build_clean_report(clean_results: &[CleanResult], skipped_count: usize) -> CleanReportState {
        let cleaned_count = clean_results.iter().map(|r| r.items_cleaned).sum();
        let bytes_freed = clean_results.iter().map(|r| r.bytes_freed).sum();
        let mut failures = Vec::new();
        let mut failure_counts: HashMap<String, usize> = HashMap::new();

        for result in clean_results {
            for (path, reason) in &result.items_failed {
                failures.push(CleanFailure {
                    category: result.category.clone(),
                    path: path.clone(),
                    reason: reason.clone(),
                });
                *failure_counts.entry(reason.clone()).or_insert(0) += 1;
            }
        }

        let failed_count = failures.len();
        let mut top_failure_reasons: Vec<(String, usize)> = failure_counts.into_iter().collect();
        top_failure_reasons.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        top_failure_reasons.truncate(5);

        CleanReportState {
            cleaned_count,
            failed_count,
            skipped_count,
            bytes_freed,
            top_failure_reasons,
            failures,
            completed_at: chrono::Utc::now(),
        }
    }

    /// Refresh dashboard data.
    fn refresh_dashboard(&mut self) {
        // Get disk space info
        if let Some(disk) = crate::disk::get_root_disk_info() {
            self.dashboard.total_disk_space = disk.total_bytes;
            self.dashboard.used_disk_space = disk.used_bytes;
            self.dashboard.free_disk_space = disk.free_bytes;
        }
    }

    /// Load disk health data.
    fn load_disk_health(&mut self) {
        self.disk_health.loading = true;
        if let Some(tx) = &self.task_tx {
            let _ = tx.send(BackgroundTask::RefreshDiskHealth);
        }
    }

    /// Save settings to config file.
    fn save_settings(&mut self) {
        let mut new_config = self.config.clone();

        for field in &self.settings.fields {
            match field.key.as_str() {
                "general.keep_recent_days" => {
                    if let SettingValue::Number(v) = &field.value {
                        new_config.general.keep_recent_days = *v as u32;
                    }
                }
                "safety.warn_threshold" => {
                    if let SettingValue::Number(v) = &field.value {
                        new_config.safety.warn_threshold = *v;
                    }
                }
                "safety.force_threshold" => {
                    if let SettingValue::Number(v) = &field.value {
                        new_config.safety.force_threshold = *v;
                    }
                }
                "safety.always_confirm" => {
                    if let SettingValue::Bool(v) = &field.value {
                        new_config.safety.always_confirm = *v;
                    }
                }
                "safety.audit_enabled" => {
                    if let SettingValue::Bool(v) = &field.value {
                        new_config.safety.audit_enabled = *v;
                    }
                }
                _ => {}
            }
        }

        new_config.tui.hide_zero_byte_items = self.clean.hide_zero_byte_items;

        let path = crate::config::default_config_path();
        let toml = match toml::to_string_pretty(&new_config) {
            Ok(content) => content,
            Err(e) => {
                self.set_status(
                    &format!("Failed to serialize settings: {}", e),
                    StatusLevel::Error,
                );
                return;
            }
        };

        if let Some(parent) = path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                self.set_status(
                    &format!("Failed to create config directory: {}", e),
                    StatusLevel::Error,
                );
                return;
            }
        }

        if let Err(e) = std::fs::write(&path, toml) {
            self.set_status(
                &format!("Failed to write settings: {}", e),
                StatusLevel::Error,
            );
            return;
        }

        self.config = new_config;
        self.clean.hide_zero_byte_items = self.config.tui.hide_zero_byte_items;
        self.audit_logger = AuditLogger::default_logger(self.config.safety.audit_enabled);
        self.settings.unsaved_changes = false;
        self.set_status("Settings saved", StatusLevel::Success);
    }

    /// Set a status message.
    fn set_status(&mut self, message: &str, level: StatusLevel) {
        self.status_message = Some((message.to_string(), level, std::time::Instant::now()));
    }

    /// Clear old status messages.
    fn clear_old_status(&mut self) {
        if let Some((_, _, time)) = &self.status_message {
            if time.elapsed() > Duration::from_secs(5) {
                self.status_message = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cleaners::{ItemType, RiskLevel};
    use std::fs;
    use tempfile::TempDir;

    fn cleanable(path: PathBuf, size: u64, risk: RiskLevel) -> SelectableItem {
        SelectableItem {
            item: CleanableItem {
                path,
                size,
                item_type: ItemType::File,
                age_days: None,
                description: "test".to_string(),
                requires_force: false,
                risk_level: risk,
            },
            category: "system-cache".to_string(),
            selected: false,
            clean_status: None,
            clean_reason: None,
        }
    }

    #[test]
    fn confirm_action_starts_cleaning_without_falling_back_to_running_mode() {
        let mut app = App::new(Config::default());
        let temp = TempDir::new().unwrap();
        let item_path = temp.path().join("cache-file");
        fs::write(&item_path, b"abc").unwrap();

        let mut row = cleanable(item_path, 3, RiskLevel::Low);
        row.selected = true;
        app.clean.items.push(row);
        app.update_selection_stats();
        app.mode = AppMode::Confirming(ConfirmAction::CleanSelected);

        app.handle_action(KeyAction::Confirm);

        assert_eq!(app.mode, AppMode::Cleaning);
        assert!(app.clean.cleaning);
        assert_eq!(
            app.clean.items[0].clean_status,
            Some(RowCleanStatus::Pending)
        );
    }

    #[test]
    fn missing_cleaner_category_is_reported_as_failure() {
        let temp = TempDir::new().unwrap();
        let item_path = temp.path().join("orphan");
        fs::write(&item_path, b"123").unwrap();

        let item = CleanableItem {
            path: item_path.clone(),
            size: 3,
            item_type: ItemType::File,
            age_days: None,
            description: "orphan".to_string(),
            requires_force: false,
            risk_level: RiskLevel::Low,
        };
        let queued = vec![QueuedCleanItem {
            category: "missing-category".to_string(),
            item,
        }];

        let results = App::do_clean(&queued, &Config::default());

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].category, "missing-category");
        assert_eq!(results[0].items_cleaned, 0);
        assert_eq!(results[0].items_failed.len(), 1);
        assert_eq!(results[0].items_failed[0].0, item_path);
    }

    #[test]
    fn zero_byte_filter_hides_rows_by_default_and_toggle_restores() {
        let mut app = App::new(Config::default());
        let temp = TempDir::new().unwrap();

        let zero = cleanable(temp.path().join("zero"), 0, RiskLevel::Low);
        let nonzero = cleanable(temp.path().join("nonzero"), 10, RiskLevel::High);
        app.clean.items = vec![zero, nonzero];

        assert!(app.clean.hide_zero_byte_items);
        assert_eq!(app.visible_clean_indices(), vec![1]);

        app.clean.hide_zero_byte_items = false;
        assert_eq!(app.visible_clean_indices(), vec![0, 1]);
    }

    #[test]
    fn clean_report_summarizes_counts_bytes_and_top_reasons() {
        let results = vec![
            CleanResult {
                category: "system-cache".to_string(),
                items_cleaned: 2,
                bytes_freed: 1024,
                items_failed: vec![
                    (PathBuf::from("/tmp/a"), "Permission denied".to_string()),
                    (PathBuf::from("/tmp/b"), "Permission denied".to_string()),
                ],
                clean_duration: Duration::ZERO,
            },
            CleanResult {
                category: "docker".to_string(),
                items_cleaned: 1,
                bytes_freed: 2048,
                items_failed: vec![(PathBuf::from("/tmp/c"), "In use".to_string())],
                clean_duration: Duration::ZERO,
            },
        ];

        let report = App::build_clean_report(&results, 1);

        assert_eq!(report.cleaned_count, 3);
        assert_eq!(report.failed_count, 3);
        assert_eq!(report.skipped_count, 1);
        assert_eq!(report.bytes_freed, 3072);
        assert_eq!(
            report.top_failure_reasons[0],
            ("Permission denied".to_string(), 2)
        );
    }
}
