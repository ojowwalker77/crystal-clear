//
//  LargeFilesDetailView.swift
//  crystalclear
//
//  Detail view for large files.
//

import SwiftUI

struct LargeFilesDetailView: View {
    @ObservedObject var viewModel: DashboardViewModel

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                VStack(alignment: .leading) {
                    Text("Large Files")
                        .font(.title2)
                        .fontWeight(.bold)
                    Text("Files larger than 100 MB")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }

                Spacer()

                if !viewModel.largeFiles.isEmpty {
                    Button("Select All") {
                        viewModel.selectAllInCategory(.largeFiles)
                    }
                    .buttonStyle(.bordered)
                }
            }
            .padding()

            Divider()

            // List
            if viewModel.largeFiles.isEmpty {
                ContentUnavailableView {
                    Label("No Large Files", systemImage: "checkmark.circle")
                } description: {
                    Text("No files larger than 100 MB found.")
                }
            } else {
                List {
                    ForEach(viewModel.largeFiles, id: \.path) { item in
                        LargeFileRow(
                            item: item,
                            isSelected: viewModel.selectedItems.contains(item.path),
                            onToggle: { viewModel.toggleItem(item.path) }
                        )
                    }
                }
                .listStyle(.inset)
            }
        }
    }
}

struct LargeFileRow: View {
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
            Image(systemName: iconForFile(item.path))
                .foregroundColor(.orange)
                .frame(width: 24)

            // Details
            VStack(alignment: .leading, spacing: 2) {
                Text((item.path as NSString).lastPathComponent)
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
            Text(CrystalClearService.shared.formatSize(item.size))
                .font(.callout)
                .fontWeight(.semibold)
                .foregroundColor(.orange)
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

    private func iconForFile(_ path: String) -> String {
        let ext = (path as NSString).pathExtension.lowercased()
        switch ext {
        case "mp4", "mov", "avi", "mkv", "webm", "m4v":
            return "film"
        case "mp3", "wav", "flac", "m4a", "aac":
            return "music.note"
        case "dmg", "pkg", "iso":
            return "opticaldisc"
        case "zip", "tar", "gz", "rar", "7z":
            return "archivebox"
        case "app":
            return "app"
        default:
            return "doc"
        }
    }
}

#Preview {
    LargeFilesDetailView(viewModel: DashboardViewModel())
}
