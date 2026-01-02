//
//  DiskUsageView.swift
//  crystalclear
//
//  View showing disk usage information.
//

import SwiftUI

// Helper to compute usage percentage
extension FfiDiskInfo {
    var usagePercent: Double {
        guard totalBytes > 0 else { return 0 }
        return Double(usedBytes) / Double(totalBytes) * 100
    }
}

struct DiskUsageView: View {
    let diskInfo: FfiDiskInfo?

    var body: some View {
        if let info = diskInfo {
            HStack(spacing: 8) {
                // Disk icon
                Image(systemName: "internaldrive")
                    .foregroundColor(.secondary)

                // Usage bar
                GeometryReader { geometry in
                    ZStack(alignment: .leading) {
                        // Background
                        RoundedRectangle(cornerRadius: 4)
                            .fill(Color.secondary.opacity(0.2))

                        // Used space
                        RoundedRectangle(cornerRadius: 4)
                            .fill(usageColor(info.usagePercent))
                            .frame(width: geometry.size.width * CGFloat(info.usagePercent) / 100)
                    }
                }
                .frame(width: 100, height: 8)

                // Percentage
                Text("\(Int(info.usagePercent))%")
                    .font(.caption)
                    .fontWeight(.medium)
                    .foregroundColor(usageColor(info.usagePercent))
                    .frame(width: 35, alignment: .trailing)

                // Free space
                Text("\(CrystalClearService.shared.formatSize(info.freeBytes)) free")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
        } else {
            Text("Disk info unavailable")
                .font(.caption)
                .foregroundColor(.secondary)
        }
    }

    private func usageColor(_ percent: Double) -> Color {
        if percent > 90 {
            return .red
        } else if percent > 75 {
            return .orange
        } else if percent > 50 {
            return .yellow
        } else {
            return .green
        }
    }
}

struct DiskDetailView: View {
    let diskInfo: FfiDiskInfo

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            HStack {
                Image(systemName: "internaldrive.fill")
                    .font(.largeTitle)
                    .foregroundColor(.blue)

                VStack(alignment: .leading) {
                    Text(diskInfo.name)
                        .font(.headline)
                    Text(diskInfo.mountPoint)
                        .font(.caption)
                        .foregroundColor(.secondary)
                }

                Spacer()

                VStack(alignment: .trailing) {
                    Text("\(Int(diskInfo.usagePercent))% used")
                        .font(.title2)
                        .fontWeight(.bold)
                        .foregroundColor(usageColor)
                }
            }

            // Large usage bar
            GeometryReader { geometry in
                ZStack(alignment: .leading) {
                    RoundedRectangle(cornerRadius: 8)
                        .fill(Color.secondary.opacity(0.2))

                    RoundedRectangle(cornerRadius: 8)
                        .fill(usageColor.gradient)
                        .frame(width: geometry.size.width * CGFloat(diskInfo.usagePercent) / 100)
                }
            }
            .frame(height: 24)

            // Stats
            HStack(spacing: 24) {
                StatItem(label: "Total", value: CrystalClearService.shared.formatSize(diskInfo.totalBytes), color: .primary)
                StatItem(label: "Used", value: CrystalClearService.shared.formatSize(diskInfo.usedBytes), color: usageColor)
                StatItem(label: "Free", value: CrystalClearService.shared.formatSize(diskInfo.freeBytes), color: .green)
            }
        }
        .padding()
        .background(Color(NSColor.controlBackgroundColor))
        .cornerRadius(12)
    }

    private var usageColor: Color {
        if diskInfo.usagePercent > 90 {
            return .red
        } else if diskInfo.usagePercent > 75 {
            return .orange
        } else if diskInfo.usagePercent > 50 {
            return .yellow
        } else {
            return .green
        }
    }
}

struct StatItem: View {
    let label: String
    let value: String
    let color: Color

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(label)
                .font(.caption)
                .foregroundColor(.secondary)
            Text(value)
                .font(.title3)
                .fontWeight(.semibold)
                .foregroundColor(color)
        }
    }
}

#Preview {
    DiskUsageView(diskInfo: nil)
        .padding()
}
