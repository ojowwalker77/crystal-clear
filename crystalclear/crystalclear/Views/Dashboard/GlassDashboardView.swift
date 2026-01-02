//
//  GlassDashboardView.swift
//  crystalclear
//
//  Beautiful glass-style dashboard inspired by Apple presentations.
//

import SwiftUI

struct GlassDashboardView: View {
    @StateObject private var viewModel = DashboardViewModel()
    @State private var showingCleanConfirmation = false
    @State private var categoryToClean: DashboardCategory?

    var body: some View {
        ZStack {
            // Background gradient
            LinearGradient(
                colors: [
                    Color(red: 0.05, green: 0.05, blue: 0.15),
                    Color(red: 0.1, green: 0.1, blue: 0.2),
                    Color(red: 0.05, green: 0.1, blue: 0.15)
                ],
                startPoint: .topLeading,
                endPoint: .bottomTrailing
            )
            .ignoresSafeArea()

            // Main content
            ScrollView {
                VStack(spacing: 24) {
                    // Header with disk usage and controls
                    GlassDiskHeader(
                        diskInfo: viewModel.diskInfo,
                        viewMode: $viewModel.viewMode,
                        isScanning: viewModel.isScanningAny,
                        onScanAll: {
                            Task { await viewModel.scanAll() }
                        }
                    )

                    // Category cards
                    if viewModel.viewMode == .normie {
                        NormieCategoryGrid(
                            viewModel: viewModel,
                            onClean: { category in
                                categoryToClean = category
                                showingCleanConfirmation = true
                            }
                        )
                    } else {
                        DeveloperCategoryGrid(viewModel: viewModel)
                    }

                    Spacer(minLength: 20)
                }
                .padding(24)
            }
        }
        .preferredColorScheme(.dark)
        .alert("Clean \(categoryToClean?.name ?? "")?", isPresented: $showingCleanConfirmation) {
            Button("Cancel", role: .cancel) {}
            Button("Clean", role: .destructive) {
                if let category = categoryToClean {
                    Task { await cleanCategory(category) }
                }
            }
        } message: {
            if let category = categoryToClean,
               let stats = viewModel.categoryStats[category],
               case .scanned(let count, let size) = stats.state {
                Text("This will move \(count) items (\(CrystalClearService.shared.formatSize(size))) to Trash.")
            }
        }
        .alert("Cleaned!", isPresented: .init(
            get: { viewModel.lastResultMessage != nil },
            set: { if !$0 { viewModel.lastResultMessage = nil } }
        )) {
            Button("OK") { viewModel.lastResultMessage = nil }
        } message: {
            Text(viewModel.lastResultMessage ?? "")
        }
    }

    private func cleanCategory(_ category: DashboardCategory) async {
        // Select all items in category and clean
        viewModel.selectAllInCategory(category)
        await viewModel.cleanSelectedItems()
    }
}

// MARK: - Glass Disk Header

struct GlassDiskHeader: View {
    let diskInfo: FfiDiskInfo?
    @Binding var viewMode: ViewMode
    let isScanning: Bool
    let onScanAll: () -> Void

    var body: some View {
        GlassCard {
            HStack(spacing: 20) {
                // Disk usage ring
                if let info = diskInfo {
                    DiskUsageRing(usagePercent: info.usagePercent, size: 80)
                }

                // Disk info
                VStack(alignment: .leading, spacing: 8) {
                    Text(diskInfo?.name ?? "Macintosh HD")
                        .font(.title2)
                        .fontWeight(.bold)
                        .foregroundColor(.white)

                    if let info = diskInfo {
                        // Usage bar
                        GeometryReader { geo in
                            ZStack(alignment: .leading) {
                                RoundedRectangle(cornerRadius: 4)
                                    .fill(Color.white.opacity(0.1))

                                RoundedRectangle(cornerRadius: 4)
                                    .fill(
                                        LinearGradient(
                                            colors: [.orange, .red],
                                            startPoint: .leading,
                                            endPoint: .trailing
                                        )
                                    )
                                    .frame(width: geo.size.width * CGFloat(info.usagePercent) / 100)
                            }
                        }
                        .frame(height: 8)

                        // Stats
                        HStack(spacing: 24) {
                            DiskStat(label: "Total", value: CrystalClearService.shared.formatSize(info.totalBytes), color: .white)
                            DiskStat(label: "Used", value: CrystalClearService.shared.formatSize(info.usedBytes), color: .orange)
                            DiskStat(label: "Free", value: CrystalClearService.shared.formatSize(info.freeBytes), color: .green)
                        }
                    }
                }

                Spacer()

                // Controls
                VStack(alignment: .trailing, spacing: 12) {
                    ViewModeToggle(mode: $viewMode)
                    ScanButton(isScanning: isScanning, action: onScanAll)
                }
            }
            .padding(24)
        }
    }
}

struct DiskStat: View {
    let label: String
    let value: String
    let color: Color

    var body: some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(label)
                .font(.caption)
                .foregroundColor(.secondary)
            Text(value)
                .font(.headline)
                .fontWeight(.semibold)
                .foregroundColor(color)
        }
    }
}

// MARK: - Normie Category Grid

struct NormieCategoryGrid: View {
    @ObservedObject var viewModel: DashboardViewModel
    let onClean: (DashboardCategory) -> Void

    private let columns = [
        GridItem(.flexible(), spacing: 16),
        GridItem(.flexible(), spacing: 16)
    ]

    var body: some View {
        LazyVGrid(columns: columns, spacing: 16) {
            ForEach(DashboardCategory.allCases) { category in
                NormieCategoryCard(
                    category: category,
                    state: viewModel.categoryStats[category]?.state ?? .notScanned,
                    onClean: { onClean(category) },
                    onScan: {
                        Task { await viewModel.scanCategory(category) }
                    }
                )
            }
        }
    }
}

struct NormieCategoryCard: View {
    let category: DashboardCategory
    let state: CategoryScanState
    let onClean: () -> Void
    let onScan: () -> Void

    var body: some View {
        GlassCard(accentColor: category.color) {
            VStack(alignment: .leading, spacing: 16) {
                // Header
                HStack {
                    GradientIconBackground(
                        icon: category.icon,
                        colors: category.gradientColors
                    )

                    Spacer()

                    // Status indicator
                    switch state {
                    case .scanning:
                        ProgressView()
                            .scaleEffect(0.8)
                    case .notScanned:
                        Button(action: onScan) {
                            Image(systemName: "arrow.clockwise")
                                .foregroundColor(.secondary)
                        }
                        .buttonStyle(.plain)
                    default:
                        EmptyView()
                    }
                }

                // Name and description
                VStack(alignment: .leading, spacing: 4) {
                    Text(category.name)
                        .font(.headline)
                        .fontWeight(.semibold)
                        .foregroundColor(.white)

                    Text(category.description)
                        .font(.caption)
                        .foregroundColor(.secondary)
                        .lineLimit(2)
                }

                Spacer()

                // Bottom: size and clean button
                HStack(alignment: .bottom) {
                    switch state {
                    case .notScanned:
                        Text("Not scanned")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    case .scanning:
                        Text("Scanning...")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    case .scanned(let itemCount, let totalSize):
                        VStack(alignment: .leading, spacing: 2) {
                            Text(CrystalClearService.shared.formatSize(totalSize))
                                .font(.title2)
                                .fontWeight(.bold)
                                .foregroundColor(category.color)
                            Text("\(itemCount) items")
                                .font(.caption)
                                .foregroundColor(.secondary)
                        }
                    case .error(let msg):
                        Text(msg)
                            .font(.caption)
                            .foregroundColor(.red)
                    }

                    Spacer()

                    if case .scanned(let count, _) = state, count > 0 {
                        Button(action: onClean) {
                            HStack(spacing: 4) {
                                Image(systemName: "sparkles")
                                    .font(.caption)
                                Text("Clean")
                                    .font(.caption)
                                    .fontWeight(.semibold)
                            }
                            .foregroundColor(.white)
                            .padding(.horizontal, 12)
                            .padding(.vertical, 8)
                            .background(
                                RoundedRectangle(cornerRadius: 8)
                                    .fill(category.color)
                            )
                        }
                        .buttonStyle(.plain)
                    }
                }
            }
            .padding(20)
            .frame(height: 200)
        }
    }
}

// MARK: - Developer Category Grid

struct DeveloperCategoryGrid: View {
    @ObservedObject var viewModel: DashboardViewModel

    var body: some View {
        VStack(spacing: 16) {
            // Category selector
            ScrollView(.horizontal, showsIndicators: false) {
                HStack(spacing: 12) {
                    ForEach(DashboardCategory.allCases) { category in
                        DevCategoryTab(
                            category: category,
                            state: viewModel.categoryStats[category]?.state ?? .notScanned,
                            isSelected: viewModel.selectedCategory == category,
                            onTap: {
                                withAnimation(.spring(response: 0.3)) {
                                    viewModel.selectedCategory = category
                                }
                            }
                        )
                    }
                }
                .padding(.horizontal, 4)
            }

            // Detail view
            if let category = viewModel.selectedCategory {
                GlassCard {
                    DevCategoryDetail(category: category, viewModel: viewModel)
                        .frame(minHeight: 400)
                }
            } else {
                GlassCard {
                    ContentUnavailableView {
                        Label("Select a Category", systemImage: "hand.tap")
                    } description: {
                        Text("Click a category tab above to see details")
                    }
                    .frame(minHeight: 400)
                }
            }
        }
    }
}

struct DevCategoryTab: View {
    let category: DashboardCategory
    let state: CategoryScanState
    let isSelected: Bool
    let onTap: () -> Void

    var body: some View {
        Button(action: onTap) {
            HStack(spacing: 8) {
                Image(systemName: category.icon)
                    .foregroundColor(isSelected ? .white : category.color)

                VStack(alignment: .leading, spacing: 0) {
                    Text(category.name)
                        .font(.caption)
                        .fontWeight(.semibold)

                    if case .scanned(_, let size) = state {
                        Text(CrystalClearService.shared.formatSize(size))
                            .font(.caption2)
                            .foregroundColor(isSelected ? .white.opacity(0.8) : .secondary)
                    }
                }
            }
            .foregroundColor(isSelected ? .white : .primary)
            .padding(.horizontal, 16)
            .padding(.vertical, 10)
            .background(
                RoundedRectangle(cornerRadius: 10)
                    .fill(isSelected ? category.color : Color.white.opacity(0.1))
            )
        }
        .buttonStyle(.plain)
    }
}

struct DevCategoryDetail: View {
    let category: DashboardCategory
    @ObservedObject var viewModel: DashboardViewModel

    var body: some View {
        switch category {
        case .systemJunk:
            DevSystemJunkList(results: viewModel.systemJunkResults, viewModel: viewModel)
        case .largeFiles:
            DevLargeFilesList(items: viewModel.largeFiles, viewModel: viewModel)
        case .duplicates:
            DevDuplicatesList(groups: viewModel.duplicateGroups, viewModel: viewModel)
        case .downloads:
            DevDownloadsList(result: viewModel.downloadsResult, viewModel: viewModel)
        case .applications:
            DevAppsList(apps: viewModel.apps, viewModel: viewModel)
        }
    }
}

// MARK: - Developer Detail Lists

struct DevSystemJunkList: View {
    let results: [FfiScanResult]
    @ObservedObject var viewModel: DashboardViewModel

    var body: some View {
        List {
            ForEach(results, id: \.category) { result in
                Section(header: Text(result.categoryName)) {
                    ForEach(result.items, id: \.path) { item in
                        DevItemRow(item: item, isSelected: viewModel.selectedItems.contains(item.path)) {
                            viewModel.toggleItem(item.path)
                        }
                    }
                }
            }
        }
        .listStyle(.plain)
        .scrollContentBackground(.hidden)
    }
}

struct DevLargeFilesList: View {
    let items: [FfiCleanableItem]
    @ObservedObject var viewModel: DashboardViewModel

    var body: some View {
        List {
            ForEach(items, id: \.path) { item in
                DevItemRow(item: item, isSelected: viewModel.selectedItems.contains(item.path)) {
                    viewModel.toggleItem(item.path)
                }
            }
        }
        .listStyle(.plain)
        .scrollContentBackground(.hidden)
    }
}

struct DevDuplicatesList: View {
    let groups: [FfiDuplicateGroup]
    @ObservedObject var viewModel: DashboardViewModel

    var body: some View {
        List {
            ForEach(groups, id: \.hash) { group in
                DisclosureGroup {
                    ForEach(Array(group.paths.enumerated()), id: \.offset) { idx, path in
                        HStack {
                            Image(systemName: idx == 0 ? "star.fill" : (viewModel.selectedItems.contains(path) ? "checkmark.circle.fill" : "circle"))
                                .foregroundColor(idx == 0 ? .yellow : (viewModel.selectedItems.contains(path) ? .accentColor : .secondary))
                            Text(path)
                                .font(.caption)
                                .lineLimit(1)
                        }
                        .contentShape(Rectangle())
                        .onTapGesture {
                            if idx > 0 { viewModel.toggleItem(path) }
                        }
                    }
                } label: {
                    HStack {
                        Image(systemName: "doc.on.doc")
                            .foregroundColor(.purple)
                        Text("\(group.count) copies")
                        Spacer()
                        Text(CrystalClearService.shared.formatSize(group.wastedSize))
                            .foregroundColor(.purple)
                    }
                }
            }
        }
        .listStyle(.plain)
        .scrollContentBackground(.hidden)
    }
}

struct DevDownloadsList: View {
    let result: FfiDownloadsScanResult?
    @ObservedObject var viewModel: DashboardViewModel

    var body: some View {
        List {
            if let result = result {
                ForEach(result.items, id: \.path) { item in
                    DevItemRow(item: item, isSelected: viewModel.selectedItems.contains(item.path)) {
                        viewModel.toggleItem(item.path)
                    }
                }
            }
        }
        .listStyle(.plain)
        .scrollContentBackground(.hidden)
    }
}

struct DevAppsList: View {
    let apps: [FfiInstalledApp]
    @ObservedObject var viewModel: DashboardViewModel

    var body: some View {
        List {
            ForEach(apps, id: \.path) { app in
                HStack(spacing: 12) {
                    AppIconView(appPath: app.path, size: 32)
                    VStack(alignment: .leading) {
                        Text(app.name)
                            .fontWeight(.medium)
                        Text(app.path)
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                    Spacer()
                    Text(CrystalClearService.shared.formatSize(app.size))
                        .foregroundColor(.red)
                }
            }
        }
        .listStyle(.plain)
        .scrollContentBackground(.hidden)
    }
}

struct DevItemRow: View {
    let item: FfiCleanableItem
    let isSelected: Bool
    let onToggle: () -> Void

    var body: some View {
        HStack(spacing: 12) {
            Image(systemName: isSelected ? "checkmark.circle.fill" : "circle")
                .foregroundColor(isSelected ? .accentColor : .secondary)
                .onTapGesture(perform: onToggle)

            VStack(alignment: .leading, spacing: 2) {
                Text((item.path as NSString).lastPathComponent)
                    .lineLimit(1)
                Text(item.path)
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .lineLimit(1)
            }

            Spacer()

            Text(CrystalClearService.shared.formatSize(item.size))
                .foregroundColor(.orange)
        }
        .contentShape(Rectangle())
        .onTapGesture(perform: onToggle)
    }
}

// MARK: - Category Extensions

extension DashboardCategory {
    var gradientColors: [Color] {
        switch self {
        case .systemJunk: return [.blue, .cyan]
        case .largeFiles: return [.orange, .yellow]
        case .duplicates: return [.purple, .pink]
        case .downloads: return [.green, .mint]
        case .applications: return [.red, .orange]
        }
    }
}

#Preview {
    GlassDashboardView()
        .frame(width: 900, height: 700)
}
