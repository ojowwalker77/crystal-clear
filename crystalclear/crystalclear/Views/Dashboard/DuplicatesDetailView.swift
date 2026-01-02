//
//  DuplicatesDetailView.swift
//  crystalclear
//
//  Detail view for duplicate files.
//

import SwiftUI

struct DuplicatesDetailView: View {
    @ObservedObject var viewModel: DashboardViewModel

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                VStack(alignment: .leading) {
                    Text("Duplicates")
                        .font(.title2)
                        .fontWeight(.bold)

                    let totalWasted = viewModel.duplicateGroups.reduce(UInt64(0)) { $0 + $1.wastedSize }
                    Text("\(viewModel.duplicateGroups.count) groups, \(CrystalClearService.shared.formatSize(totalWasted)) wasted")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }

                Spacer()
            }
            .padding()

            Divider()

            // List
            if viewModel.duplicateGroups.isEmpty {
                ContentUnavailableView {
                    Label("No Duplicates", systemImage: "checkmark.circle")
                } description: {
                    Text("No duplicate files found.")
                }
            } else {
                List {
                    ForEach(viewModel.duplicateGroups, id: \.hash) { group in
                        DuplicateGroupRow(
                            group: group,
                            selectedItems: viewModel.selectedItems,
                            onToggle: { path in
                                viewModel.toggleItem(path)
                            }
                        )
                    }
                }
                .listStyle(.inset)
            }
        }
    }
}

struct DuplicateGroupRow: View {
    let group: FfiDuplicateGroup
    let selectedItems: Set<String>
    let onToggle: (String) -> Void

    @State private var isExpanded = false

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            // Group header
            Button {
                withAnimation {
                    isExpanded.toggle()
                }
            } label: {
                HStack(spacing: 12) {
                    Image(systemName: isExpanded ? "chevron.down" : "chevron.right")
                        .foregroundColor(.secondary)
                        .frame(width: 16)

                    Image(systemName: "doc.on.doc.fill")
                        .foregroundColor(.purple)

                    VStack(alignment: .leading, spacing: 2) {
                        Text("\(group.count) copies")
                            .fontWeight(.medium)

                        Text("Each \(CrystalClearService.shared.formatSize(group.size))")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }

                    Spacer()

                    VStack(alignment: .trailing, spacing: 2) {
                        Text(CrystalClearService.shared.formatSize(group.wastedSize))
                            .fontWeight(.semibold)
                            .foregroundColor(.purple)

                        Text("wasted")
                            .font(.caption2)
                            .foregroundColor(.secondary)
                    }
                }
                .padding(.vertical, 8)
            }
            .buttonStyle(.plain)

            // Expanded file list
            if isExpanded {
                VStack(spacing: 4) {
                    ForEach(Array(group.paths.enumerated()), id: \.offset) { index, path in
                        HStack(spacing: 12) {
                            // Checkbox (first one is "original", others can be deleted)
                            if index == 0 {
                                Image(systemName: "star.fill")
                                    .foregroundColor(.yellow)
                                    .font(.caption)
                                    .frame(width: 20)
                            } else {
                                Image(systemName: selectedItems.contains(path) ? "checkmark.circle.fill" : "circle")
                                    .foregroundColor(selectedItems.contains(path) ? .accentColor : .secondary)
                                    .font(.caption)
                                    .frame(width: 20)
                                    .onTapGesture {
                                        onToggle(path)
                                    }
                            }

                            Text(path)
                                .font(.caption)
                                .foregroundColor(.secondary)
                                .lineLimit(1)
                                .truncationMode(.middle)

                            Spacer()

                            if index == 0 {
                                Text("Keep")
                                    .font(.caption2)
                                    .foregroundColor(.green)
                                    .padding(.horizontal, 6)
                                    .padding(.vertical, 2)
                                    .background(Color.green.opacity(0.2))
                                    .cornerRadius(4)
                            }
                        }
                        .padding(.leading, 28)
                        .padding(.vertical, 2)
                        .contentShape(Rectangle())
                        .onTapGesture {
                            if index > 0 {
                                onToggle(path)
                            }
                        }
                    }
                }
                .padding(.bottom, 8)
            }
        }
    }
}

#Preview {
    DuplicatesDetailView(viewModel: DashboardViewModel())
}
