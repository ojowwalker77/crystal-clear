//
//  DiskUsageHeader.swift
//  crystalclear
//
//  Large disk usage header for the dashboard.
//

import SwiftUI

struct DiskUsageHeader: View {
    let diskInfo: FfiDiskInfo?
    let potentialSavings: String
    let isScanning: Bool
    let onScanAll: () -> Void

    var body: some View {
        VStack(spacing: 16) {
            if let info = diskInfo {
                // Disk name and icon
                HStack(spacing: 12) {
                    Image(systemName: "internaldrive.fill")
                        .font(.system(size: 40))
                        .foregroundColor(.blue)

                    VStack(alignment: .leading, spacing: 4) {
                        Text(info.name)
                            .font(.title2)
                            .fontWeight(.semibold)

                        Text(info.mountPoint)
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }

                    Spacer()

                    // Scan All button
                    Button(action: onScanAll) {
                        HStack {
                            if isScanning {
                                ProgressView()
                                    .scaleEffect(0.8)
                                    .progressViewStyle(CircularProgressViewStyle())
                            } else {
                                Image(systemName: "magnifyingglass")
                            }
                            Text(isScanning ? "Scanning..." : "Scan All")
                        }
                        .font(.headline)
                        .foregroundColor(.white)
                        .padding(.horizontal, 20)
                        .padding(.vertical, 10)
                        .background(isScanning ? Color.gray : Color.blue)
                        .cornerRadius(10)
                    }
                    .buttonStyle(.plain)
                    .disabled(isScanning)
                }

                // Large usage bar
                GeometryReader { geometry in
                    ZStack(alignment: .leading) {
                        RoundedRectangle(cornerRadius: 10)
                            .fill(Color.secondary.opacity(0.2))

                        RoundedRectangle(cornerRadius: 10)
                            .fill(usageGradient(for: info.usagePercent))
                            .frame(width: geometry.size.width * CGFloat(info.usagePercent) / 100)
                    }
                }
                .frame(height: 24)

                // Stats row
                HStack(spacing: 0) {
                    StatBox(
                        label: "Total",
                        value: CrystalClearService.shared.formatSize(info.totalBytes),
                        color: .primary
                    )

                    Spacer()

                    StatBox(
                        label: "Used",
                        value: CrystalClearService.shared.formatSize(info.usedBytes),
                        color: usageColor(for: info.usagePercent)
                    )

                    Spacer()

                    StatBox(
                        label: "Free",
                        value: CrystalClearService.shared.formatSize(info.freeBytes),
                        color: .green
                    )

                    Spacer()

                    StatBox(
                        label: "Can Free",
                        value: potentialSavings,
                        color: .orange
                    )
                }
            } else {
                HStack {
                    ProgressView()
                    Text("Loading disk info...")
                        .foregroundColor(.secondary)
                }
            }
        }
        .padding(20)
        .background(Color(NSColor.controlBackgroundColor))
        .cornerRadius(16)
    }

    private func usageColor(for percent: Double) -> Color {
        if percent > 90 { return .red }
        if percent > 75 { return .orange }
        if percent > 50 { return .yellow }
        return .green
    }

    private func usageGradient(for percent: Double) -> LinearGradient {
        let color = usageColor(for: percent)
        return LinearGradient(
            colors: [color.opacity(0.8), color],
            startPoint: .leading,
            endPoint: .trailing
        )
    }
}

struct StatBox: View {
    let label: String
    let value: String
    let color: Color

    var body: some View {
        VStack(alignment: .center, spacing: 4) {
            Text(label)
                .font(.caption)
                .foregroundColor(.secondary)
            Text(value)
                .font(.title3)
                .fontWeight(.semibold)
                .foregroundColor(color)
        }
        .frame(minWidth: 80)
    }
}

#Preview {
    DiskUsageHeader(
        diskInfo: nil,
        potentialSavings: "2.5 GB",
        isScanning: false,
        onScanAll: {}
    )
    .padding()
}
