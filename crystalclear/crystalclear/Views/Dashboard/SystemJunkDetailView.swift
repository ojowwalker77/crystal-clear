//
//  SystemJunkDetailView.swift
//  crystalclear
//
//  Detail view for system junk (caches, logs, etc.).
//

import SwiftUI

struct SystemJunkDetailView: View {
    @ObservedObject var viewModel: DashboardViewModel

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                VStack(alignment: .leading) {
                    Text("System Junk")
                        .font(.title2)
                        .fontWeight(.bold)

                    let totalItems = viewModel.systemJunkResults.reduce(0) { $0 + $1.items.count }
                    let totalSize = viewModel.systemJunkResults.reduce(UInt64(0)) { $0 + $1.totalSize }
                    Text("\(totalItems) items, \(CrystalClearService.shared.formatSize(totalSize))")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }

                Spacer()

                if !viewModel.systemJunkResults.isEmpty {
                    Button("Select All") {
                        viewModel.selectAllInCategory(.systemJunk)
                    }
                    .buttonStyle(.bordered)
                }
            }
            .padding()

            Divider()

            // List grouped by category
            if viewModel.systemJunkResults.isEmpty {
                ContentUnavailableView {
                    Label("No Junk", systemImage: "checkmark.circle")
                } description: {
                    Text("Your system is clean!")
                }
            } else {
                List {
                    ForEach(viewModel.systemJunkResults, id: \.category) { result in
                        if !result.items.isEmpty {
                            Section {
                                ForEach(result.items, id: \.path) { item in
                                    SystemJunkItemRow(
                                        item: item,
                                        isSelected: viewModel.selectedItems.contains(item.path),
                                        onToggle: { viewModel.toggleItem(item.path) }
                                    )
                                }
                            } header: {
                                HStack {
                                    Text(result.categoryName)
                                        .font(.headline)
                                    Spacer()
                                    Text(CrystalClearService.shared.formatSize(result.totalSize))
                                        .font(.caption)
                                        .foregroundColor(.secondary)
                                }
                            }
                        }
                    }
                }
                .listStyle(.inset)
            }
        }
    }
}

struct SystemJunkItemRow: View {
    let item: FfiCleanableItem
    let isSelected: Bool
    let onToggle: () -> Void

    @State private var isHovering = false

    var body: some View {
        HStack(spacing: 12) {
            // Checkbox
            Image(systemName: isSelected ? "checkmark.circle.fill" : "circle")
                .foregroundColor(isSelected ? .accentColor : .secondary)
                .font(.title3)
                .onTapGesture(perform: onToggle)

            // Icon based on type
            Image(systemName: iconForType(item.itemType))
                .foregroundColor(colorForRisk(item.riskLevel))
                .frame(width: 20)

            // Details
            VStack(alignment: .leading, spacing: 2) {
                Text(item.description.isEmpty ? (item.path as NSString).lastPathComponent : item.description)
                    .fontWeight(.medium)
                    .lineLimit(1)

                Text(item.path)
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .lineLimit(1)
                    .truncationMode(.middle)
            }

            Spacer()

            // Size and risk
            VStack(alignment: .trailing) {
                Text(CrystalClearService.shared.formatSize(item.size))
                    .font(.callout)
                    .fontWeight(.medium)
                    .foregroundColor(.blue)

                RiskBadge(level: item.riskLevel)
            }
        }
        .padding(.vertical, 4)
        .contentShape(Rectangle())
        .onTapGesture(perform: onToggle)
        .background(isHovering ? Color.accentColor.opacity(0.1) : Color.clear)
        .cornerRadius(6)
        .onHover { hovering in
            isHovering = hovering
        }
    }

    private func iconForType(_ type: FfiItemType) -> String {
        switch type {
        case .file:
            return "doc"
        case .directory:
            return "folder"
        case .symlink:
            return "link"
        }
    }

    private func colorForRisk(_ risk: FfiRiskLevel) -> Color {
        switch risk {
        case .low:
            return .green
        case .medium:
            return .orange
        case .high:
            return .red
        }
    }
}

#Preview {
    SystemJunkDetailView(viewModel: DashboardViewModel())
}
