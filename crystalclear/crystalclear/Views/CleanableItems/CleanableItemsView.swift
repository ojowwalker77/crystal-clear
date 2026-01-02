//
//  CleanableItemsView.swift
//  crystalclear
//
//  View showing cleanable items for a selected category.
//

import SwiftUI

struct CleanableItemsView: View {
    @ObservedObject var viewModel: MainViewModel

    var body: some View {
        VStack(spacing: 0) {
            if viewModel.isScanning {
                ProgressView("Scanning...")
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else if let category = viewModel.selectedCategory,
                      case .cleaner(let id) = category,
                      let result = viewModel.scanResults[id] {
                if result.items.isEmpty {
                    ContentUnavailableView {
                        Label("No Items Found", systemImage: "checkmark.circle")
                    } description: {
                        Text("This category is clean!")
                    }
                } else {
                    ItemsList(items: result.items, viewModel: viewModel)
                }
            } else {
                ContentUnavailableView {
                    Label("Select a Category", systemImage: "sidebar.left")
                } description: {
                    Text("Choose a category from the sidebar to see cleanable items.")
                }
            }
        }
        .frame(minWidth: 300)
    }
}

struct ItemsList: View {
    let items: [FfiCleanableItem]
    @ObservedObject var viewModel: MainViewModel

    var body: some View {
        List {
            ForEach(items, id: \.path) { item in
                CleanableItemRow(
                    item: item,
                    isSelected: viewModel.selectedItems.contains(item.path),
                    onToggle: { viewModel.toggleItem(item.path) }
                )
            }
        }
        .listStyle(.inset)
    }
}

struct CleanableItemRow: View {
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

            // Icon
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

                if let ageDays = item.ageDays, ageDays > 0 {
                    Text("\(ageDays) days old")
                        .font(.caption2)
                        .foregroundStyle(.tertiary)
                }
            }

            Spacer()

            // Size
            VStack(alignment: .trailing) {
                Text(CrystalClearService.shared.formatSize(item.size))
                    .font(.callout)
                    .fontWeight(.medium)
                    .foregroundColor(.orange)

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

struct RiskBadge: View {
    let level: FfiRiskLevel

    var body: some View {
        Text(label)
            .font(.caption2)
            .fontWeight(.medium)
            .padding(.horizontal, 6)
            .padding(.vertical, 2)
            .background(color.opacity(0.2))
            .foregroundColor(color)
            .cornerRadius(4)
    }

    private var label: String {
        switch level {
        case .low:
            return "Low"
        case .medium:
            return "Medium"
        case .high:
            return "High"
        }
    }

    private var color: Color {
        switch level {
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
    CleanableItemsView(viewModel: MainViewModel())
}
