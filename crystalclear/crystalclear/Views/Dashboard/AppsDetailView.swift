//
//  AppsDetailView.swift
//  crystalclear
//
//  Detail view for applications with real app icons.
//

import SwiftUI

struct AppsDetailView: View {
    @ObservedObject var viewModel: DashboardViewModel
    @State private var selectedApps: Set<String> = []
    @State private var showConfirmation = false

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                VStack(alignment: .leading) {
                    Text("Applications")
                        .font(.title2)
                        .fontWeight(.bold)

                    let totalSize = viewModel.apps.reduce(UInt64(0)) { $0 + $1.size }
                    let leftoverSize = viewModel.apps.reduce(UInt64(0)) { $0 + $1.leftoverSize }
                    Text("\(viewModel.apps.count) apps (\(CrystalClearService.shared.formatSize(totalSize)) + \(CrystalClearService.shared.formatSize(leftoverSize)) leftovers)")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }

                Spacer()

                if !selectedApps.isEmpty {
                    Button("Uninstall Selected (\(selectedApps.count))") {
                        showConfirmation = true
                    }
                    .buttonStyle(.borderedProminent)
                    .tint(.red)
                }
            }
            .padding()

            Divider()

            // List
            if viewModel.apps.isEmpty {
                ContentUnavailableView {
                    Label("No Applications", systemImage: "square.grid.2x2")
                } description: {
                    Text("Click 'Scan' to find installed applications.")
                }
            } else {
                List {
                    ForEach(viewModel.apps, id: \.path) { app in
                        AppDetailRow(
                            app: app,
                            isSelected: selectedApps.contains(app.path),
                            onToggle: {
                                if selectedApps.contains(app.path) {
                                    selectedApps.remove(app.path)
                                } else {
                                    selectedApps.insert(app.path)
                                }
                            }
                        )
                    }
                }
                .listStyle(.inset)
            }
        }
        .alert("Uninstall Applications?", isPresented: $showConfirmation) {
            Button("Cancel", role: .cancel) {}
            Button("Uninstall", role: .destructive) {
                Task {
                    await uninstallSelectedApps()
                }
            }
        } message: {
            Text("This will move \(selectedApps.count) application(s) and their leftovers to the Trash.")
        }
    }

    private func uninstallSelectedApps() async {
        for appPath in selectedApps {
            _ = await CrystalClearService.shared.uninstallApp(appPath: appPath, includeLeftovers: true)
        }
        selectedApps.removeAll()
        await viewModel.scanApplications()
    }
}

struct AppDetailRow: View {
    let app: FfiInstalledApp
    let isSelected: Bool
    let onToggle: () -> Void

    @State private var isHovering = false
    @State private var showLeftovers = false

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack(spacing: 12) {
                // Checkbox
                Image(systemName: isSelected ? "checkmark.circle.fill" : "circle")
                    .foregroundColor(isSelected ? .red : .secondary)
                    .font(.title3)
                    .onTapGesture(perform: onToggle)

                // Real app icon
                AppIconView(appPath: app.path, size: 40)

                // App details
                VStack(alignment: .leading, spacing: 2) {
                    Text(app.name)
                        .fontWeight(.medium)

                    if let bundleId = app.bundleId {
                        Text(bundleId)
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }

                    if !app.leftoverPaths.isEmpty {
                        Button {
                            withAnimation {
                                showLeftovers.toggle()
                            }
                        } label: {
                            HStack(spacing: 4) {
                                Image(systemName: showLeftovers ? "chevron.down" : "chevron.right")
                                    .font(.caption2)
                                Text("\(app.leftoverPaths.count) leftovers")
                            }
                            .font(.caption)
                            .foregroundColor(.orange)
                        }
                        .buttonStyle(.plain)
                    }
                }

                Spacer()

                // Size
                VStack(alignment: .trailing, spacing: 2) {
                    Text(CrystalClearService.shared.formatSize(app.size))
                        .font(.callout)
                        .fontWeight(.medium)
                        .foregroundColor(.red)

                    if app.leftoverSize > 0 {
                        Text("+\(CrystalClearService.shared.formatSize(app.leftoverSize))")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                }
            }
            .padding(.vertical, 4)
            .contentShape(Rectangle())
            .onTapGesture(perform: onToggle)
            .background(isHovering ? Color.red.opacity(0.1) : Color.clear)
            .cornerRadius(6)
            .onHover { hovering in
                isHovering = hovering
            }

            // Leftovers list
            if showLeftovers && !app.leftoverPaths.isEmpty {
                VStack(alignment: .leading, spacing: 4) {
                    ForEach(app.leftoverPaths, id: \.self) { leftover in
                        HStack(spacing: 8) {
                            Image(systemName: "folder")
                                .foregroundColor(.secondary)
                                .font(.caption)
                            Text(leftover)
                                .font(.caption)
                                .foregroundColor(.secondary)
                                .lineLimit(1)
                                .truncationMode(.middle)
                        }
                        .padding(.leading, 64)
                    }
                }
                .padding(.vertical, 4)
            }
        }
    }
}

#Preview {
    AppsDetailView(viewModel: DashboardViewModel())
}
