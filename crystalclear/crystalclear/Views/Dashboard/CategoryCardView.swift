//
//  CategoryCardView.swift
//  crystalclear
//
//  Individual category card for the dashboard.
//

import SwiftUI

struct CategoryCardView: View {
    let category: DashboardCategory
    let state: CategoryScanState
    let isSelected: Bool
    let onTap: () -> Void
    let onScan: () -> Void

    var body: some View {
        Button(action: onTap) {
            VStack(alignment: .leading, spacing: 12) {
                // Header with icon and scan button
                HStack {
                    // Category icon
                    ZStack {
                        Circle()
                            .fill(category.color.opacity(0.15))
                            .frame(width: 48, height: 48)

                        Image(systemName: category.icon)
                            .font(.title2)
                            .foregroundColor(category.color)
                    }

                    Spacer()

                    // Scan button or status
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

                // Category name
                Text(category.name)
                    .font(.headline)
                    .foregroundColor(.primary)

                // Description
                Text(category.description)
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .lineLimit(2)

                Spacer()

                // Stats
                HStack {
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
                                .font(.title3)
                                .fontWeight(.bold)
                                .foregroundColor(category.color)

                            Text("\(itemCount) items")
                                .font(.caption)
                                .foregroundColor(.secondary)
                        }
                    case .error(let message):
                        Text(message)
                            .font(.caption)
                            .foregroundColor(.red)
                    }

                    Spacer()

                    if case .scanned = state {
                        Image(systemName: "chevron.right")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                }
            }
            .padding(16)
            .frame(height: 180)
            .background(
                RoundedRectangle(cornerRadius: 16)
                    .fill(Color(NSColor.controlBackgroundColor))
                    .shadow(color: isSelected ? category.color.opacity(0.3) : Color.black.opacity(0.1), radius: isSelected ? 8 : 4)
            )
            .overlay(
                RoundedRectangle(cornerRadius: 16)
                    .stroke(isSelected ? category.color : Color.clear, lineWidth: 2)
            )
        }
        .buttonStyle(.plain)
    }
}

#Preview {
    HStack(spacing: 16) {
        CategoryCardView(
            category: .systemJunk,
            state: .notScanned,
            isSelected: false,
            onTap: {},
            onScan: {}
        )

        CategoryCardView(
            category: .largeFiles,
            state: .scanned(itemCount: 42, totalSize: 5_368_709_120),
            isSelected: true,
            onTap: {},
            onScan: {}
        )

        CategoryCardView(
            category: .duplicates,
            state: .scanning,
            isSelected: false,
            onTap: {},
            onScan: {}
        )
    }
    .padding()
}
