//
//  CrystalClearService.swift
//  crystalclear
//
//  Service wrapper for the Crystal Clear Rust core.
//

import Foundation

/// Singleton service that wraps the Rust core functionality.
/// All Rust FFI calls go through this service.
@MainActor
final class CrystalClearService: ObservableObject {
    static let shared = CrystalClearService()

    @Published private(set) var isScanning = false
    @Published private(set) var isCleaning = false
    @Published private(set) var lastError: String?

    private init() {}

    // MARK: - Scanning

    /// Scan all available categories for cleanable items.
    func scanAllCategories(dryRun: Bool = true, keepRecentDays: UInt32 = 7, minSize: UInt64? = nil, maxItems: UInt32? = nil) async -> [FfiScanResult] {
        isScanning = true
        lastError = nil
        defer { isScanning = false }

        let options = FfiScanOptions(
            dryRun: dryRun,
            keepRecentDays: keepRecentDays,
            minSize: minSize,
            maxItems: maxItems
        )

        return await Task.detached {
            crystalclear.scanAllCategories(options: options)
        }.value
    }

    /// Scan a specific category for cleanable items.
    func scanCategory(categoryId: String, dryRun: Bool = true, keepRecentDays: UInt32 = 7, minSize: UInt64? = nil, maxItems: UInt32? = nil) async -> FfiScanResult {
        isScanning = true
        lastError = nil
        defer { isScanning = false }

        let options = FfiScanOptions(
            dryRun: dryRun,
            keepRecentDays: keepRecentDays,
            minSize: minSize,
            maxItems: maxItems
        )

        return await Task.detached {
            crystalclear.scanCategory(categoryId: categoryId, options: options)
        }.value
    }

    /// Scan for installed applications.
    func scanApps() async -> [FfiInstalledApp] {
        isScanning = true
        lastError = nil
        defer { isScanning = false }

        return await Task.detached {
            crystalclear.scanApplications()
        }.value
    }

    // MARK: - Cleaning

    /// Clean the specified items (move to trash).
    func cleanItems(paths: [String], dryRun: Bool = false, force: Bool = false) async -> FfiCleanResult {
        isCleaning = true
        lastError = nil
        defer { isCleaning = false }

        let options = FfiCleanOptions(dryRun: dryRun, force: force)

        return await Task.detached {
            crystalclear.cleanItems(paths: paths, options: options)
        }.value
    }

    /// Uninstall an application and optionally its leftovers.
    func uninstallApp(appPath: String, includeLeftovers: Bool = true) async -> FfiCleanResult {
        isCleaning = true
        lastError = nil
        defer { isCleaning = false }

        return await Task.detached {
            crystalclear.uninstallApp(appPath: appPath, includeLeftovers: includeLeftovers)
        }.value
    }

    // MARK: - Disk Info

    /// Get root disk information.
    func getRootDiskInfo() -> FfiDiskInfo? {
        crystalclear.getRootDiskInfo()
    }

    /// Get all disk information.
    func getAllDiskInfo() -> [FfiDiskInfo] {
        crystalclear.getAllDiskInfo()
    }

    // MARK: - New Scanners

    /// Scan for large files.
    func scanLargeFiles(minSizeBytes: UInt64 = 100 * 1024 * 1024, maxResults: UInt32 = 100, includeHidden: Bool = false) async -> [FfiCleanableItem] {
        isScanning = true
        lastError = nil
        defer { isScanning = false }

        let options = FfiLargeFileScanOptions(
            minSizeBytes: minSizeBytes,
            maxResults: maxResults,
            includeHidden: includeHidden
        )

        return await Task.detached {
            crystalclear.scanLargeFiles(options: options)
        }.value
    }

    /// Scan for duplicate files.
    func scanDuplicates(minSizeBytes: UInt64 = 1024 * 1024, maxGroups: UInt32 = 50) async -> FfiDuplicateScanResult {
        isScanning = true
        lastError = nil
        defer { isScanning = false }

        let options = FfiDuplicateScanOptions(
            minSizeBytes: minSizeBytes,
            maxGroups: maxGroups
        )

        return await Task.detached {
            crystalclear.scanDuplicates(options: options)
        }.value
    }

    /// Scan downloads folder.
    func scanDownloads(oldThresholdDays: UInt32 = 30, includeHidden: Bool = false) async -> FfiDownloadsScanResult {
        isScanning = true
        lastError = nil
        defer { isScanning = false }

        let options = FfiDownloadsScanOptions(
            oldThresholdDays: oldThresholdDays,
            includeHidden: includeHidden
        )

        return await Task.detached {
            crystalclear.scanDownloads(options: options)
        }.value
    }

    // MARK: - Utilities

    /// Get all available cleaners.
    func getAvailableCleaners() -> [FfiCleanerInfo] {
        crystalclear.getAvailableCleaners()
    }

    /// Check if a path is protected.
    func isProtectedPath(_ path: String) -> Bool {
        crystalclear.isProtectedPath(path: path)
    }

    /// Format a byte size for display.
    func formatSize(_ bytes: UInt64) -> String {
        crystalclear.formatSize(bytes: bytes)
    }
}
