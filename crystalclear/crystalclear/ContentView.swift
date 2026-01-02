//
//  ContentView.swift
//  crystalclear
//
//  Simple, friendly interface
//

import SwiftUI

// MARK: - Colors

private let bgBlack = Color.black
private let bgHover = Color.white.opacity(0.08)
private let textWhite = Color.white
private let textGray = Color.white.opacity(0.5)
private let accentGreen = Color(red: 0.35, green: 0.8, blue: 0.55)

// MARK: - Main View

struct ContentView: View {
    @StateObject private var vm = DashboardViewModel()
    @State private var showCleanConfirm = false
    @State private var hoveredPath: String?
    @State private var collapsedSections: Set<String> = []

    private var hasScanned: Bool {
        vm.categoryStats.values.contains { if case .scanned = $0.state { return true }; return false }
    }

    var body: some View {
        ZStack {
            bgBlack.ignoresSafeArea()

            VStack(spacing: 0) {
                if !hasScanned && !vm.isScanningAny {
                    welcomeScreen
                } else {
                    mainScreen
                }
            }
        }
        .confirmationDialog("Move \(vm.selectedItems.count) items to Trash?", isPresented: $showCleanConfirm) {
            Button("Move to Trash", role: .destructive) {
                Task { await vm.cleanSelectedItems() }
            }
            Button("Cancel", role: .cancel) {}
        }
        .alert("Done", isPresented: .init(
            get: { vm.lastResultMessage != nil },
            set: { if !$0 { vm.lastResultMessage = nil } }
        )) {} message: {
            Text(vm.lastResultMessage ?? "")
        }
    }

    // MARK: - Welcome Screen

    private var welcomeScreen: some View {
        VStack(spacing: 32) {
            Spacer()

            // Big icon
            ZStack {
                Circle()
                    .fill(accentGreen.opacity(0.15))
                    .frame(width: 120, height: 120)
                Image(systemName: "sparkles")
                    .font(.system(size: 48))
                    .foregroundColor(accentGreen)
            }

            // Title
            VStack(spacing: 8) {
                Text("Crystal Clear")
                    .font(.system(size: 28, weight: .bold))
                    .foregroundColor(textWhite)

                Text("Free up space on your Mac")
                    .font(.system(size: 16))
                    .foregroundColor(textGray)
            }

            // Disk info
            if let info = vm.diskInfo {
                HStack(spacing: 8) {
                    Image(systemName: "internaldrive")
                        .foregroundColor(textGray)
                    Text("\(fmt(info.freeBytes)) available of \(fmt(info.totalBytes))")
                        .foregroundColor(textGray)
                }
                .font(.system(size: 14))
            }

            // Big scan button
            Button {
                Task { await vm.scanAll() }
            } label: {
                HStack(spacing: 10) {
                    Image(systemName: "magnifyingglass")
                        .font(.system(size: 16, weight: .semibold))
                    Text("Scan for Junk Files")
                        .font(.system(size: 16, weight: .semibold))
                }
                .foregroundColor(.black)
                .padding(.horizontal, 32)
                .padding(.vertical, 14)
                .background(accentGreen)
                .clipShape(RoundedRectangle(cornerRadius: 12))
            }
            .buttonStyle(.plain)

            Spacer()

            // Footer hint
            Text("Finds caches, large files, duplicates, and old downloads")
                .font(.system(size: 13))
                .foregroundColor(textGray)
                .padding(.bottom, 24)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    // MARK: - Main Screen (After Scan)

    private var mainScreen: some View {
        VStack(spacing: 0) {
            // Header with big savings number
            header

            // List of items
            ScrollView {
                LazyVStack(spacing: 0, pinnedViews: .sectionHeaders) {
                    let sysItems = vm.systemJunkResults.flatMap { $0.items }
                    let largeItems = vm.largeFiles
                    let dupItems = duplicateItems
                    let dlItems = vm.downloadsResult?.items ?? []

                    if !sysItems.isEmpty { section("System Junk", sysItems) }
                    if !largeItems.isEmpty { section("Large Files", largeItems) }
                    if !dupItems.isEmpty { section("Duplicates", dupItems) }
                    if !dlItems.isEmpty { section("Downloads", dlItems) }

                    if vm.isScanningAny && sysItems.isEmpty && largeItems.isEmpty {
                        scanningIndicator
                    }
                }
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)

            // Footer with clean button
            if !vm.selectedItems.isEmpty {
                footer
            }
        }
    }

    private var header: some View {
        HStack {
            VStack(alignment: .leading, spacing: 4) {
                if vm.isScanningAny {
                    HStack(spacing: 8) {
                        ProgressView()
                            .scaleEffect(0.6)
                        Text("Scanning...")
                            .font(.system(size: 14))
                            .foregroundColor(textGray)
                    }
                } else {
                    Text(fmt(vm.totalPotentialSavings))
                        .font(.system(size: 32, weight: .bold))
                        .foregroundColor(textWhite)
                    Text("can be cleaned")
                        .font(.system(size: 14))
                        .foregroundColor(textGray)
                }
            }

            Spacer()

            if let info = vm.diskInfo {
                VStack(alignment: .trailing, spacing: 4) {
                    Text(fmt(info.freeBytes))
                        .font(.system(size: 16, weight: .medium))
                        .foregroundColor(textWhite)
                    Text("free space")
                        .font(.system(size: 12))
                        .foregroundColor(textGray)
                }
            }

            Button {
                Task { await vm.scanAll() }
            } label: {
                Image(systemName: "arrow.triangle.2.circlepath")
                    .font(.system(size: 14))
                    .foregroundColor(textGray)
                    .frame(width: 36, height: 36)
                    .background(Color.white.opacity(0.1))
                    .clipShape(Circle())
            }
            .buttonStyle(.plain)
            .disabled(vm.isScanningAny)
            .padding(.leading, 16)
        }
        .padding(24)
    }

    private var duplicateItems: [FfiCleanableItem] {
        vm.duplicateGroups.flatMap { group in
            group.paths.dropFirst().map { path in
                FfiCleanableItem(
                    path: path, size: group.size, itemType: .file,
                    ageDays: nil, description: "Duplicate",
                    requiresForce: false, riskLevel: .low
                )
            }
        }
    }

    @ViewBuilder
    private func section(_ title: String, _ items: [FfiCleanableItem]) -> some View {
        let totalSize = items.reduce(UInt64(0)) { $0 + $1.size }
        let selectedCount = items.filter { vm.selectedItems.contains($0.path) }.count
        let isCollapsed = collapsedSections.contains(title)

        Section {
            if !isCollapsed {
                ForEach(items, id: \.path) { item in
                    itemRow(item)
                }
            }
        } header: {
            HStack {
                // Collapse toggle
                Button {
                    withAnimation(.easeInOut(duration: 0.2)) {
                        if isCollapsed {
                            collapsedSections.remove(title)
                        } else {
                            collapsedSections.insert(title)
                        }
                    }
                } label: {
                    Image(systemName: isCollapsed ? "chevron.right" : "chevron.down")
                        .font(.system(size: 11, weight: .semibold))
                        .foregroundColor(textGray)
                        .frame(width: 16)
                }
                .buttonStyle(.plain)

                Text(title)
                    .font(.system(size: 13, weight: .semibold))
                    .foregroundColor(textWhite)

                Text("\(items.count) items")
                    .font(.system(size: 12))
                    .foregroundColor(textGray)

                Spacer()

                Text(fmt(totalSize))
                    .font(.system(size: 13, weight: .medium))
                    .foregroundColor(accentGreen)

                // Select all / Deselect all
                Button {
                    if selectedCount == items.count {
                        for item in items { vm.selectedItems.remove(item.path) }
                    } else {
                        for item in items { vm.selectedItems.insert(item.path) }
                    }
                } label: {
                    Text(selectedCount == items.count ? "Deselect" : "Select all")
                        .font(.system(size: 11, weight: .medium))
                        .foregroundColor(textGray)
                        .padding(.horizontal, 8)
                        .padding(.vertical, 4)
                        .background(Color.white.opacity(0.08))
                        .clipShape(RoundedRectangle(cornerRadius: 4))
                }
                .buttonStyle(.plain)
            }
            .padding(.horizontal, 24)
            .padding(.vertical, 12)
            .background(bgBlack)
            .contentShape(Rectangle())
            .onTapGesture {
                withAnimation(.easeInOut(duration: 0.2)) {
                    if isCollapsed {
                        collapsedSections.remove(title)
                    } else {
                        collapsedSections.insert(title)
                    }
                }
            }
        }
    }

    private func itemRow(_ item: FfiCleanableItem) -> some View {
        let isSelected = vm.selectedItems.contains(item.path)
        let isHovered = hoveredPath == item.path

        return Button {
            if isSelected { vm.selectedItems.remove(item.path) }
            else { vm.selectedItems.insert(item.path) }
        } label: {
            HStack(spacing: 14) {
                // Checkbox
                ZStack {
                    RoundedRectangle(cornerRadius: 5)
                        .fill(isSelected ? accentGreen : Color.clear)
                        .frame(width: 20, height: 20)
                    RoundedRectangle(cornerRadius: 5)
                        .stroke(isSelected ? accentGreen : Color.white.opacity(0.3), lineWidth: 2)
                        .frame(width: 20, height: 20)
                    if isSelected {
                        Image(systemName: "checkmark")
                            .font(.system(size: 11, weight: .bold))
                            .foregroundColor(.black)
                    }
                }

                // Name
                Text((item.path as NSString).lastPathComponent)
                    .font(.system(size: 14))
                    .foregroundColor(textWhite)
                    .lineLimit(1)

                Spacer()

                // Size
                Text(fmt(item.size))
                    .font(.system(size: 13).monospacedDigit())
                    .foregroundColor(textGray)
            }
            .padding(.horizontal, 24)
            .padding(.vertical, 10)
            .background(isHovered ? bgHover : Color.clear)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .onHover { hoveredPath = $0 ? item.path : nil }
    }

    private var scanningIndicator: some View {
        VStack(spacing: 16) {
            ProgressView()
                .scaleEffect(1.2)
            Text("Looking for junk files...")
                .font(.system(size: 14))
                .foregroundColor(textGray)
        }
        .frame(maxWidth: .infinity)
        .padding(.vertical, 60)
    }

    private var footer: some View {
        HStack {
            // Selection info
            VStack(alignment: .leading, spacing: 2) {
                Text("\(vm.selectedItems.count) items selected")
                    .font(.system(size: 14, weight: .medium))
                    .foregroundColor(textWhite)
                Text(vm.formattedSelectedSize)
                    .font(.system(size: 24, weight: .bold))
                    .foregroundColor(accentGreen)
            }

            Spacer()

            // Clean button
            Button {
                showCleanConfirm = true
            } label: {
                HStack(spacing: 8) {
                    Image(systemName: "trash")
                        .font(.system(size: 14, weight: .semibold))
                    Text("Clean")
                        .font(.system(size: 15, weight: .semibold))
                }
                .foregroundColor(.black)
                .padding(.horizontal, 28)
                .padding(.vertical, 12)
                .background(accentGreen)
                .clipShape(RoundedRectangle(cornerRadius: 10))
            }
            .buttonStyle(.plain)
        }
        .padding(24)
        .background(Color.white.opacity(0.03))
    }

    private func fmt(_ bytes: UInt64) -> String {
        CrystalClearService.shared.formatSize(bytes)
    }
}

#Preview {
    ContentView()
        .frame(width: 800, height: 600)
}
