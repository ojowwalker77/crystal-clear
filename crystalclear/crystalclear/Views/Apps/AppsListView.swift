//
//  AppsListView.swift
//  crystalclear
//
//  View showing installed applications.
//

import SwiftUI

struct AppsListView: View {
    @ObservedObject var viewModel: MainViewModel

    var body: some View {
        VStack(spacing: 0) {
            if viewModel.isScanning {
                ProgressView("Scanning applications...")
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else if viewModel.apps.isEmpty {
                ContentUnavailableView {
                    Label("No Applications", systemImage: "square.grid.2x2")
                } description: {
                    Text("Click 'Scan Apps' to find installed applications.")
                }
            } else {
                List {
                    ForEach(viewModel.apps, id: \.path) { app in
                        AppRow(
                            app: app,
                            isSelected: viewModel.selectedApps.contains(app.path),
                            onToggle: { viewModel.toggleApp(app.path) }
                        )
                    }
                }
                .listStyle(.inset)
            }
        }
        .frame(minWidth: 300)
    }
}

struct AppRow: View {
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
                    .foregroundColor(isSelected ? .accentColor : .secondary)
                    .font(.title3)
                    .onTapGesture(perform: onToggle)

                // App icon (placeholder)
                RoundedRectangle(cornerRadius: 8)
                    .fill(Color.blue.gradient)
                    .frame(width: 40, height: 40)
                    .overlay(
                        Text(String(app.name.prefix(1)))
                            .font(.title2)
                            .fontWeight(.bold)
                            .foregroundColor(.white)
                    )

                // App details
                VStack(alignment: .leading, spacing: 2) {
                    Text(app.name)
                        .fontWeight(.medium)

                    Text(app.path)
                        .font(.caption)
                        .foregroundColor(.secondary)
                        .lineLimit(1)
                        .truncationMode(.middle)

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
                        .foregroundColor(.orange)

                    if !app.leftoverPaths.isEmpty {
                        Text("+\(CrystalClearService.shared.formatSize(app.leftoverSize))")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
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
                        .padding(.leading, 52)
                    }
                }
                .padding(.vertical, 4)
            }
        }
    }
}

#Preview {
    AppsListView(viewModel: MainViewModel())
}
