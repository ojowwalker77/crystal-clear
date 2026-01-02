//
//  DownloadsDetailView.swift
//  crystalclear
//
//  Detail view for downloads.
//

import SwiftUI

struct DownloadsDetailView: View {
    @ObservedObject var viewModel: DashboardViewModel

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                VStack(alignment: .leading) {
                    Text("Downloads")
                        .font(.title2)
                        .fontWeight(.bold)

                    if let result = viewModel.downloadsResult {
                        Text("\(result.items.count) items, \(CrystalClearService.shared.formatSize(result.totalSize))")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                }

                Spacer()

                if let result = viewModel.downloadsResult, !result.items.isEmpty {
                    Button("Select All") {
                        viewModel.selectAllInCategory(.downloads)
                    }
                    .buttonStyle(.bordered)
                }
            }
            .padding()

            // Category breakdown
            if let result = viewModel.downloadsResult {
                ScrollView(.horizontal, showsIndicators: false) {
                    HStack(spacing: 12) {
                        ForEach(result.sizeByCategory, id: \.category) { catSize in
                            CategoryChip(
                                category: catSize.category,
                                size: catSize.size
                            )
                        }
                    }
                    .padding(.horizontal)
                }
                .padding(.bottom, 8)
            }

            Divider()

            // List
            if let result = viewModel.downloadsResult, !result.items.isEmpty {
                List {
                    ForEach(result.items, id: \.path) { item in
                        DownloadItemRow(
                            item: item,
                            isSelected: viewModel.selectedItems.contains(item.path),
                            onToggle: { viewModel.toggleItem(item.path) }
                        )
                    }
                }
                .listStyle(.inset)
            } else {
                ContentUnavailableView {
                    Label("No Downloads", systemImage: "checkmark.circle")
                } description: {
                    Text("Your Downloads folder is clean!")
                }
            }
        }
    }
}

struct CategoryChip: View {
    let category: FfiDownloadCategory
    let size: UInt64

    var body: some View {
        HStack(spacing: 6) {
            Image(systemName: iconForCategory)
                .foregroundColor(colorForCategory)

            VStack(alignment: .leading, spacing: 0) {
                Text(nameForCategory)
                    .font(.caption)
                    .fontWeight(.medium)
                Text(CrystalClearService.shared.formatSize(size))
                    .font(.caption2)
                    .foregroundColor(.secondary)
            }
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 8)
        .background(colorForCategory.opacity(0.1))
        .cornerRadius(8)
    }

    private var nameForCategory: String {
        switch category {
        case .oldFiles: return "Old Files"
        case .archives: return "Archives"
        case .largeMedia: return "Large Media"
        case .documents: return "Documents"
        case .installers: return "Installers"
        case .other: return "Other"
        }
    }

    private var iconForCategory: String {
        switch category {
        case .oldFiles: return "clock"
        case .archives: return "archivebox"
        case .largeMedia: return "film"
        case .documents: return "doc"
        case .installers: return "opticaldisc"
        case .other: return "questionmark.folder"
        }
    }

    private var colorForCategory: Color {
        switch category {
        case .oldFiles: return .gray
        case .archives: return .brown
        case .largeMedia: return .purple
        case .documents: return .blue
        case .installers: return .orange
        case .other: return .secondary
        }
    }
}

struct DownloadItemRow: View {
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
            Image(systemName: iconForFile)
                .foregroundColor(.green)
                .frame(width: 24)

            // Details
            VStack(alignment: .leading, spacing: 2) {
                Text((item.path as NSString).lastPathComponent)
                    .fontWeight(.medium)
                    .lineLimit(1)

                Text(item.description)
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .lineLimit(1)

                if let ageDays = item.ageDays, ageDays > 0 {
                    Text("\(ageDays) days old")
                        .font(.caption2)
                        .foregroundStyle(.tertiary)
                }
            }

            Spacer()

            // Size
            Text(CrystalClearService.shared.formatSize(item.size))
                .font(.callout)
                .fontWeight(.semibold)
                .foregroundColor(.green)
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

    private var iconForFile: String {
        let ext = (item.path as NSString).pathExtension.lowercased()
        switch ext {
        case "dmg", "pkg", "iso":
            return "opticaldisc"
        case "zip", "tar", "gz", "rar", "7z":
            return "archivebox"
        case "mp4", "mov", "avi", "mkv":
            return "film"
        case "pdf", "doc", "docx":
            return "doc.text"
        default:
            return "arrow.down.circle"
        }
    }
}

#Preview {
    DownloadsDetailView(viewModel: DashboardViewModel())
}
