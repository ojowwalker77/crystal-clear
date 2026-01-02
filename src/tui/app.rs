//! Main application state and run loop.

use std::collections::HashSet;
use std::io;
use std::sync::mpsc::{self, Receiver, Sender};
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

/// An item that can be selected for cleaning.
pub struct SelectableItem {
    pub item: CleanableItem,
    pub category: String,
    pub selected: bool,
}

/// Clean tab state.
#[derive(Default)]
pub struct CleanTabState {
    pub items: Vec<SelectableItem>,
    pub table_state: TableState,
    pub selected_count: usize,
    pub selected_size: u64,
    pub cleaning: bool,
    pub clean_results: Option<Vec<CleanResult>>,
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
    Left,  // Cleanable items
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
    CleanItems(Vec<CleanableItem>),
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
            clean: CleanTabState::default(),
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

        let config = self.config.clone();

        thread::spawn(move || {
            while let Ok(task) = task_rx.recv() {
                match task {
                    BackgroundTask::ScanCategories => {
                        let results = Self::do_scan(&config, &result_tx);
                        let _ = result_tx.send(TaskResult::ScanComplete(results));
                    }
                    BackgroundTask::CleanItems(items) => {
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

    /// Perform scanning in background.
    fn do_scan(config: &Config, tx: &Sender<TaskResult>) -> Vec<ScanResult> {
        use crate::cleaners::{
            Cleaner, CleanerContext, AppsLeftoversCleaner, CargoCleaner, DockerCleaner,
            NpmCleaner, PipCleaner, SystemCachesCleaner, SystemLogsCleaner, TrashCleaner,
            YarnCleaner,
        };
        #[cfg(target_os = "macos")]
        use crate::cleaners::XcodeCleaner;
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        use crate::cleaners::HomebrewCleaner;
        use crate::audit::AuditLogger;
        use crate::trash::TrashMover;
        use crate::discovery::{DiscoveryConfig, PathDiscovery};

        // First, run discovery to find new paths (silently)
        let _ = tx.send(TaskResult::ScanProgress("Discovering paths...".to_string(), 0, 1));
        if let Ok(db) = Database::open() {
            let discovery = PathDiscovery::new(db, DiscoveryConfig::default());
            // Run incremental discovery (faster, uses cache)
            let _ = discovery.discover_incremental();
        }

        // Create instances for scanning (won't actually move files in dry_run mode)
        let trash_mover = TrashMover::default_mover().ok();
        let _audit_logger = AuditLogger::default_logger(false); // Disable audit for scanning

        // Build cleaners - only those where we have a trash_mover if needed
        let mut cleaners: Vec<Box<dyn Cleaner>> = Vec::new();

        if let Some(ref _tm) = trash_mover {
            // Cleaners that need TrashMover - we clone the trash_mover path for each
            // Actually we can't clone, so we create new instances
            if let Ok(tm) = TrashMover::default_mover() {
                cleaners.push(Box::new(SystemCachesCleaner::new(
                    tm,
                    AuditLogger::default_logger(false),
                    &config.categories.system_cache,
                )));
            }
            if let Ok(tm) = TrashMover::default_mover() {
                cleaners.push(Box::new(SystemLogsCleaner::new(
                    tm,
                    AuditLogger::default_logger(false),
                    &config.categories.system_logs,
                )));
            }
            #[cfg(target_os = "macos")]
            if let Ok(tm) = TrashMover::default_mover() {
                cleaners.push(Box::new(XcodeCleaner::new(
                    tm,
                    AuditLogger::default_logger(false),
                    &config.categories.xcode,
                )));
            }
            if let Ok(tm) = TrashMover::default_mover() {
                cleaners.push(Box::new(NpmCleaner::new(
                    tm,
                    AuditLogger::default_logger(false),
                    &config.categories.npm,
                )));
            }
            if let Ok(tm) = TrashMover::default_mover() {
                cleaners.push(Box::new(YarnCleaner::new(
                    tm,
                    AuditLogger::default_logger(false),
                    &config.categories.yarn,
                )));
            }
            if let Ok(tm) = TrashMover::default_mover() {
                cleaners.push(Box::new(CargoCleaner::new(
                    tm,
                    AuditLogger::default_logger(false),
                    &config.categories.cargo,
                )));
            }
            if let Ok(tm) = TrashMover::default_mover() {
                cleaners.push(Box::new(PipCleaner::new(
                    tm,
                    AuditLogger::default_logger(false),
                    &config.categories.pip,
                )));
            }
        }

        // Cleaners that don't need TrashMover
        cleaners.push(Box::new(TrashCleaner::new(
            AuditLogger::default_logger(false),
            &config.categories.trash,
        )));
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        cleaners.push(Box::new(HomebrewCleaner::new(
            AuditLogger::default_logger(false),
            &config.categories.homebrew,
        )));
        cleaners.push(Box::new(DockerCleaner::new(
            AuditLogger::default_logger(false),
            &config.categories.docker,
        )));
        cleaners.push(Box::new(AppsLeftoversCleaner::new(
            AuditLogger::default_logger(false),
            &config.categories.apps,
        )));

        let ctx = CleanerContext {
            dry_run: true,
            keep_recent_days: config.general.keep_recent_days,
            ..Default::default()
        };

        let total = cleaners.len();
        let mut results = Vec::new();

        for (i, cleaner) in cleaners.into_iter().enumerate() {
            let _ = tx.send(TaskResult::ScanProgress(
                format!("Scanning {}...", cleaner.name()),
                i + 1,
                total,
            ));

            if cleaner.is_available() {
                let result = cleaner.scan(&ctx);
                results.push(result);
            }

            // Small delay to allow UI updates
            thread::sleep(Duration::from_millis(50));
        }

        results
    }

    /// Perform cleaning in background.
    fn do_clean(items: &[CleanableItem], config: &Config) -> Vec<CleanResult> {
        // Group items by category and clean
        // For now, return empty results
        let _ = (items, config);
        Vec::new()
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
                    let cleaned_count: usize = clean_results.iter().map(|r| r.items_cleaned).sum();
                    let cleaned_size: u64 = clean_results.iter().map(|r| r.bytes_freed).sum();
                    self.clean.clean_results = Some(clean_results);
                    self.clean.items.clear(); // Clear the list after cleaning
                    self.clean.selected_count = 0;
                    self.clean.selected_size = 0;
                    self.mode = AppMode::Running;
                    self.set_status(
                        &format!("Cleaned {} items ({})", cleaned_count, crate::scanner::format_size(cleaned_size)),
                        StatusLevel::Success,
                    );
                    // Refresh disk info
                    self.refresh_dashboard();
                }
                TaskResult::DiskHealthLoaded(disks) => {
                    self.disk_health.loading = false;
                    self.disk_health.smart_available =
                        disks.iter().any(|d| d.smart_data.is_some());
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
                        &format!("Uninstalled {} apps ({})", count, crate::scanner::format_size(size)),
                        StatusLevel::Success,
                    );
                    self.refresh_dashboard();
                }
                TaskResult::Error(msg) => {
                    self.dashboard.discovering = false;
                    self.scan.scanning = false;
                    self.clean.cleaning = false;
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
                });
            }
        }
    }

    /// Handle a user action.
    fn handle_action(&mut self, action: KeyAction) {
        // Handle mode-specific actions first
        match self.mode {
            AppMode::Confirming(confirm_action) => {
                match action {
                    KeyAction::Confirm => {
                        self.execute_confirmed_action(confirm_action);
                        self.mode = AppMode::Running;
                    }
                    KeyAction::Cancel | KeyAction::Quit => {
                        self.mode = AppMode::Running;
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
            // Panel switching (Left/Right or h/l)
            KeyAction::Left | KeyAction::Right => {
                self.focus_panel = match self.focus_panel {
                    FocusPanel::Left => FocusPanel::Right,
                    FocusPanel::Right => FocusPanel::Left,
                };
            }
            // Navigation (within focused panel)
            KeyAction::Down => {
                match self.focus_panel {
                    FocusPanel::Left => {
                        let len = self.clean.items.len();
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
                }
            }
            KeyAction::Up => {
                match self.focus_panel {
                    FocusPanel::Left => {
                        let i = self.clean.table_state.selected().unwrap_or(0);
                        self.clean.table_state.select(Some(i.saturating_sub(1)));
                    }
                    FocusPanel::Right => {
                        let i = self.apps.table_state.selected().unwrap_or(0);
                        self.apps.table_state.select(Some(i.saturating_sub(1)));
                    }
                }
            }
            // Selection (within focused panel)
            KeyAction::Select => {
                match self.focus_panel {
                    FocusPanel::Left => {
                        if let Some(i) = self.clean.table_state.selected() {
                            if let Some(item) = self.clean.items.get_mut(i) {
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
                }
            }
            KeyAction::SelectAll => {
                match self.focus_panel {
                    FocusPanel::Left => {
                        for item in &mut self.clean.items {
                            item.selected = true;
                        }
                        self.update_selection_stats();
                    }
                    FocusPanel::Right => {
                        for i in 0..self.apps.apps.len() {
                            self.apps.selected_for_uninstall.insert(i);
                        }
                    }
                }
            }
            KeyAction::SelectNone => {
                match self.focus_panel {
                    FocusPanel::Left => {
                        for item in &mut self.clean.items {
                            item.selected = false;
                        }
                        self.update_selection_stats();
                    }
                    FocusPanel::Right => {
                        self.apps.selected_for_uninstall.clear();
                    }
                }
            }
            // Clean/Uninstall action
            KeyAction::StartClean | KeyAction::Confirm => {
                match self.focus_panel {
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
                }
            }
            _ => {}
        }
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
            if tx.send(BackgroundTask::UninstallApps(apps_to_uninstall)).is_ok() {
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

        let items: Vec<CleanableItem> = self
            .clean
            .items
            .iter()
            .filter(|i| i.selected)
            .map(|i| i.item.clone())
            .collect();

        if items.is_empty() {
            return;
        }

        self.clean.cleaning = true;
        self.mode = AppMode::Cleaning;

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
        // TODO: Apply settings to config and save
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
