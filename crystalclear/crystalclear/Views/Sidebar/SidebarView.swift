//
//  SidebarView.swift
//  crystalclear
//
//  Sidebar showing available cleaners and categories.
//

import SwiftUI

struct SidebarView: View {
    @ObservedObject var viewModel: MainViewModel

    var body: some View {
        List(selection: $viewModel.selectedCategory) {
            Section("Cleaners") {
                ForEach(viewModel.cleaners, id: \.id) { cleaner in
                    CleanerRow(
                        cleaner: cleaner,
                        scanResult: viewModel.scanResults[cleaner.id]
                    )
                    .tag(CategorySelection.cleaner(cleaner.id))
                }
            }

            Section("Applications") {
                Label {
                    HStack {
                        Text("Installed Apps")
                        Spacer()
                        if !viewModel.apps.isEmpty {
                            Text("\(viewModel.apps.count)")
                                .foregroundColor(.secondary)
                                .font(.caption)
                        }
                    }
                } icon: {
                    Image(systemName: "square.grid.2x2")
                        .foregroundColor(.blue)
                }
                .tag(CategorySelection.apps)
            }
        }
        .listStyle(.sidebar)
        .navigationTitle("Crystal Clear")
    }
}

struct CleanerRow: View {
    let cleaner: FfiCleanerInfo
    let scanResult: FfiScanResult?

    var body: some View {
        Label {
            HStack {
                VStack(alignment: .leading, spacing: 2) {
                    Text(cleaner.name)
                    if let result = scanResult, !result.items.isEmpty {
                        Text("\(result.items.count) items")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                }
                Spacer()
                if let result = scanResult, result.totalSize > 0 {
                    Text(formatSize(result.totalSize))
                        .font(.caption)
                        .foregroundColor(.orange)
                }
            }
        } icon: {
            Image(systemName: iconForCategory(cleaner.category))
                .foregroundColor(colorForCategory(cleaner.category))
        }
        .opacity(cleaner.available ? 1.0 : 0.5)
    }

    private func formatSize(_ bytes: UInt64) -> String {
        CrystalClearService.shared.formatSize(bytes)
    }

    private func iconForCategory(_ category: FfiCleanerCategory) -> String {
        switch category {
        case .system:
            return "folder.badge.gearshape"
        case .developer:
            return "hammer"
        case .apps:
            return "app.badge.checkmark"
        }
    }

    private func colorForCategory(_ category: FfiCleanerCategory) -> Color {
        switch category {
        case .system:
            return .orange
        case .developer:
            return .purple
        case .apps:
            return .blue
        }
    }
}

#Preview {
    SidebarView(viewModel: MainViewModel())
}
