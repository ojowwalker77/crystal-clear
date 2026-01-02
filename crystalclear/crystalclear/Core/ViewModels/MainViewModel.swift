//
//  MainViewModel.swift
//  crystalclear
//
//  Main view model managing application state.
//

import Foundation
import SwiftUI

/// Category selection for the sidebar.
enum CategorySelection: Hashable {
    case cleaner(String)  // cleaner ID
    case apps
}

/// Main view model for the Crystal Clear app.
@MainActor
final class MainViewModel: ObservableObject {
    // MARK: - Published Properties

    /// Available cleaners
    @Published private(set) var cleaners: [FfiCleanerInfo] = []

    /// Current scan results by category
    @Published private(set) var scanResults: [String: FfiScanResult] = [:]

    /// Currently selected category in sidebar
    @Published var selectedCategory: CategorySelection? = nil

    /// Selected item paths for cleaning
    @Published var selectedItems: Set<String> = []

    /// Installed applications
    @Published private(set) var apps: [FfiInstalledApp] = []

    /// Selected apps for uninstall
    @Published var selectedApps: Set<String> = []

    /// Root disk info
    @Published private(set) var diskInfo: FfiDiskInfo?

    /// Status message
    @Published private(set) var statusMessage: String = "Ready"

    /// Whether app is scanning
    @Published private(set) var isScanning: Bool = false

    /// Whether app is cleaning
    @Published private(set) var isCleaning: Bool = false

    /// Last operation result message
    @Published var lastResultMessage: String?

    // MARK: - Private Properties

    private let service = CrystalClearService.shared

    // MARK: - Computed Properties

    /// Total selected size for cleaning
    var totalSelectedSize: UInt64 {
        var size: UInt64 = 0
        for (_, result) in scanResults {
            for item in result.items where selectedItems.contains(item.path) {
                size += item.size
            }
        }
        return size
    }

    /// Total selected items count
    var selectedItemCount: Int {
        selectedItems.count
    }

    /// Formatted selected size
    var formattedSelectedSize: String {
        service.formatSize(totalSelectedSize)
    }

    /// Items for the currently selected category
    var currentCategoryItems: [FfiCleanableItem] {
        guard case .cleaner(let id) = selectedCategory,
              let result = scanResults[id] else {
            return []
        }
        return result.items
    }

    // MARK: - Initialization

    init() {
        loadCleaners()
        refreshDiskInfo()
    }

    // MARK: - Public Methods

    /// Load available cleaners.
    func loadCleaners() {
        cleaners = service.getAvailableCleaners()
    }

    /// Refresh disk info.
    func refreshDiskInfo() {
        diskInfo = service.getRootDiskInfo()
    }

    /// Scan all categories.
    func scanAll() async {
        isScanning = true
        statusMessage = "Scanning..."
        selectedItems.removeAll()

        let results = await service.scanAllCategories()

        scanResults.removeAll()
        for result in results {
            scanResults[result.category] = result
        }

        let totalItems = results.reduce(0) { $0 + $1.items.count }
        let totalSize = results.reduce(UInt64(0)) { $0 + $1.totalSize }

        statusMessage = "Found \(totalItems) items (\(service.formatSize(totalSize)))"
        isScanning = false
    }

    /// Scan applications.
    func scanApps() async {
        isScanning = true
        statusMessage = "Scanning applications..."
        selectedApps.removeAll()

        apps = await service.scanApps()

        statusMessage = "Found \(apps.count) applications"
        isScanning = false
    }

    /// Clean selected items.
    func cleanSelectedItems() async {
        guard !selectedItems.isEmpty else { return }

        isCleaning = true
        let count = selectedItems.count
        statusMessage = "Cleaning \(count) items..."

        let result = await service.cleanItems(paths: Array(selectedItems))

        if result.failures.isEmpty {
            lastResultMessage = "Cleaned \(result.itemsCleaned) items, freed \(service.formatSize(result.bytesFreed))"
        } else {
            lastResultMessage = "Cleaned \(result.itemsCleaned) items with \(result.failures.count) failures"
        }

        // Remove cleaned items from selection
        selectedItems.removeAll()

        // Refresh scan results
        await scanAll()
        refreshDiskInfo()

        statusMessage = "Ready"
        isCleaning = false
    }

    /// Uninstall selected apps.
    func uninstallSelectedApps(includeLeftovers: Bool = true) async {
        guard !selectedApps.isEmpty else { return }

        isCleaning = true
        let count = selectedApps.count
        statusMessage = "Uninstalling \(count) apps..."

        var totalFreed: UInt64 = 0
        var totalCleaned: UInt32 = 0
        var totalFailures = 0

        for appPath in selectedApps {
            let result = await service.uninstallApp(appPath: appPath, includeLeftovers: includeLeftovers)
            totalFreed += result.bytesFreed
            totalCleaned += result.itemsCleaned
            totalFailures += result.failures.count
        }

        if totalFailures == 0 {
            lastResultMessage = "Uninstalled \(totalCleaned) items, freed \(service.formatSize(totalFreed))"
        } else {
            lastResultMessage = "Uninstalled with \(totalFailures) failures"
        }

        selectedApps.removeAll()

        // Refresh apps list
        await scanApps()
        refreshDiskInfo()

        statusMessage = "Ready"
        isCleaning = false
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

    /// Select all items in current category.
    func selectAllInCategory() {
        for item in currentCategoryItems {
            selectedItems.insert(item.path)
        }
    }

    /// Deselect all items.
    func deselectAll() {
        selectedItems.removeAll()
    }

    /// Toggle app selection.
    func toggleApp(_ path: String) {
        if selectedApps.contains(path) {
            selectedApps.remove(path)
        } else {
            selectedApps.insert(path)
        }
    }
}
