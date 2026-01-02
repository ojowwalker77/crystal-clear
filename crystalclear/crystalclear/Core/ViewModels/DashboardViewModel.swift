//
//  DashboardViewModel.swift
//  crystalclear
//
//  Main view model for the dashboard.
//

import Foundation
import SwiftUI

/// Category type for the dashboard
enum DashboardCategory: String, CaseIterable, Identifiable {
    case systemJunk = "system"
    case largeFiles = "large"
    case duplicates = "duplicates"
    case downloads = "downloads"
    case applications = "apps"

    var id: String { rawValue }

    var name: String {
        switch self {
        case .systemJunk: return "System Junk"
        case .largeFiles: return "Large Files"
        case .duplicates: return "Duplicates"
        case .downloads: return "Downloads"
        case .applications: return "Applications"
        }
    }

    var icon: String {
        switch self {
        case .systemJunk: return "folder.badge.gearshape"
        case .largeFiles: return "externaldrive"
        case .duplicates: return "doc.on.doc"
        case .downloads: return "arrow.down.circle"
        case .applications: return "square.grid.2x2"
        }
    }

    var color: Color {
        switch self {
        case .systemJunk: return .blue
        case .largeFiles: return .orange
        case .duplicates: return .purple
        case .downloads: return .green
        case .applications: return .red
        }
    }

    var description: String {
        switch self {
        case .systemJunk: return "Caches, logs, and temporary files"
        case .largeFiles: return "Files over 100 MB"
        case .duplicates: return "Duplicate files wasting space"
        case .downloads: return "Old downloads and installers"
        case .applications: return "Installed apps and leftovers"
        }
    }
}

/// Scan state for a category
enum CategoryScanState {
    case notScanned
    case scanning
    case scanned(itemCount: Int, totalSize: UInt64)
    case error(String)
}

/// Stats for a category card
struct CategoryStats {
    var state: CategoryScanState = .notScanned
    var items: [FfiCleanableItem] = []
}

/// Main view model for the dashboard
@MainActor
final class DashboardViewModel: ObservableObject {
    // MARK: - Published Properties

    /// View mode (normie vs developer)
    @Published var viewMode: ViewMode = .normie

    /// Currently selected category
    @Published var selectedCategory: DashboardCategory? = nil

    /// Scan states for each category
    @Published private(set) var categoryStats: [DashboardCategory: CategoryStats] = [:]

    /// Disk info
    @Published private(set) var diskInfo: FfiDiskInfo?

    /// Whether any scan is in progress
    @Published private(set) var isScanningAny: Bool = false

    /// Large files results
    @Published private(set) var largeFiles: [FfiCleanableItem] = []

    /// Duplicate groups
    @Published private(set) var duplicateGroups: [FfiDuplicateGroup] = []

    /// Downloads results
    @Published private(set) var downloadsResult: FfiDownloadsScanResult?

    /// System junk results
    @Published private(set) var systemJunkResults: [FfiScanResult] = []

    /// Apps list
    @Published private(set) var apps: [FfiInstalledApp] = []

    /// Selected items for cleaning
    @Published var selectedItems: Set<String> = []

    /// Last result message
    @Published var lastResultMessage: String?

    /// Whether cleaning is in progress
    @Published private(set) var isCleaning: Bool = false

    // MARK: - Private Properties

    private let service = CrystalClearService.shared

    // MARK: - Computed Properties

    /// Total potential savings across all categories (excludes apps)
    var totalPotentialSavings: UInt64 {
        var total: UInt64 = 0
        for (category, stats) in categoryStats {
            // Don't count apps in potential savings
            if category == .applications { continue }
            if case .scanned(_, let size) = stats.state {
                total += size
            }
        }
        return total
    }

    /// Formatted total savings
    var formattedTotalSavings: String {
        service.formatSize(totalPotentialSavings)
    }

    /// Total selected size
    var totalSelectedSize: UInt64 {
        var size: UInt64 = 0
        // Sum from large files
        for item in largeFiles where selectedItems.contains(item.path) {
            size += item.size
        }
        // Sum from downloads
        if let downloads = downloadsResult {
            for item in downloads.items where selectedItems.contains(item.path) {
                size += item.size
            }
        }
        // Sum from system junk
        for result in systemJunkResults {
            for item in result.items where selectedItems.contains(item.path) {
                size += item.size
            }
        }
        return size
    }

    /// Formatted selected size
    var formattedSelectedSize: String {
        service.formatSize(totalSelectedSize)
    }

    // MARK: - Initialization

    init() {
        // Initialize stats for all categories
        for category in DashboardCategory.allCases {
            categoryStats[category] = CategoryStats()
        }
        refreshDiskInfo()
    }

    // MARK: - Public Methods

    /// Refresh disk info.
    func refreshDiskInfo() {
        diskInfo = service.getRootDiskInfo()
    }

    /// Scan all categories.
    func scanAll() async {
        isScanningAny = true

        // Scan all in parallel
        async let systemTask: () = scanSystemJunk()
        async let largeTask: () = scanLargeFiles()
        async let duplicatesTask: () = scanDuplicates()
        async let downloadsTask: () = scanDownloads()
        async let appsTask: () = scanApplications()

        _ = await (systemTask, largeTask, duplicatesTask, downloadsTask, appsTask)

        refreshDiskInfo()
        isScanningAny = false
    }

    /// Scan a specific category.
    func scanCategory(_ category: DashboardCategory) async {
        switch category {
        case .systemJunk:
            await scanSystemJunk()
        case .largeFiles:
            await scanLargeFiles()
        case .duplicates:
            await scanDuplicates()
        case .downloads:
            await scanDownloads()
        case .applications:
            await scanApplications()
        }
    }

    /// Scan system junk.
    func scanSystemJunk() async {
        categoryStats[.systemJunk]?.state = .scanning
        let results = await service.scanAllCategories()
        systemJunkResults = results

        let totalItems = results.reduce(0) { $0 + $1.items.count }
        let totalSize = results.reduce(UInt64(0)) { $0 + $1.totalSize }

        // Auto-select all items
        for result in results {
            for item in result.items {
                selectedItems.insert(item.path)
            }
        }

        categoryStats[.systemJunk]?.state = .scanned(itemCount: totalItems, totalSize: totalSize)
    }

    /// Scan large files.
    func scanLargeFiles() async {
        categoryStats[.largeFiles]?.state = .scanning
        largeFiles = await service.scanLargeFiles()

        // Auto-select all items
        for item in largeFiles {
            selectedItems.insert(item.path)
        }

        let totalSize = largeFiles.reduce(UInt64(0)) { $0 + $1.size }
        categoryStats[.largeFiles]?.state = .scanned(itemCount: largeFiles.count, totalSize: totalSize)
    }

    /// Scan duplicates.
    func scanDuplicates() async {
        categoryStats[.duplicates]?.state = .scanning
        let result = await service.scanDuplicates()
        duplicateGroups = result.groups

        // Auto-select all duplicate items (except first in each group)
        for group in result.groups {
            for path in group.paths.dropFirst() {
                selectedItems.insert(path)
            }
        }

        categoryStats[.duplicates]?.state = .scanned(
            itemCount: result.groups.count,
            totalSize: result.totalWasted
        )
    }

    /// Scan downloads.
    func scanDownloads() async {
        categoryStats[.downloads]?.state = .scanning
        let result = await service.scanDownloads()
        downloadsResult = result

        // Don't auto-select downloads - user might want to keep them

        categoryStats[.downloads]?.state = .scanned(
            itemCount: result.items.count,
            totalSize: result.totalSize
        )
    }

    /// Scan applications.
    func scanApplications() async {
        categoryStats[.applications]?.state = .scanning
        apps = await service.scanApps()

        let totalSize = apps.reduce(UInt64(0)) { $0 + $1.size + $1.leftoverSize }
        categoryStats[.applications]?.state = .scanned(itemCount: apps.count, totalSize: totalSize)
    }

    /// Clean selected items.
    func cleanSelectedItems() async {
        guard !selectedItems.isEmpty else { return }

        isCleaning = true
        let paths = Array(selectedItems)

        let result = await service.cleanItems(paths: paths)

        if result.failures.isEmpty {
            lastResultMessage = "Cleaned \(result.itemsCleaned) items, freed \(service.formatSize(result.bytesFreed))"
        } else {
            lastResultMessage = "Cleaned \(result.itemsCleaned) items with \(result.failures.count) failures"
        }

        selectedItems.removeAll()
        isCleaning = false

        // Refresh data
        await scanAll()
    }

    // MARK: - Selection Helpers

    /// Toggle selection of an item.
    func toggleItem(_ path: String) {
        if selectedItems.contains(path) {
            selectedItems.remove(path)
        } else {
            selectedItems.insert(path)
        }
    }

    /// Select all items in a category.
    func selectAllInCategory(_ category: DashboardCategory) {
        switch category {
        case .largeFiles:
            for item in largeFiles {
                selectedItems.insert(item.path)
            }
        case .downloads:
            if let result = downloadsResult {
                for item in result.items {
                    selectedItems.insert(item.path)
                }
            }
        case .systemJunk:
            for result in systemJunkResults {
                for item in result.items {
                    selectedItems.insert(item.path)
                }
            }
        default:
            break
        }
    }

    /// Deselect all items.
    func deselectAll() {
        selectedItems.removeAll()
    }
}
