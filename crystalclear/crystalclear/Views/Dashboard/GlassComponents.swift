//
//  GlassComponents.swift
//  crystalclear
//
//  Frosted glass UI components inspired by Apple presentation style.
//

import SwiftUI

// MARK: - View Mode

enum ViewMode: String, CaseIterable {
    case normie = "Simple"
    case developer = "Developer"

    var icon: String {
        switch self {
        case .normie: return "sparkles"
        case .developer: return "terminal"
        }
    }
}

// MARK: - Glass Card Background

struct GlassCard<Content: View>: View {
    let content: Content
    var isSelected: Bool = false
    var accentColor: Color = .blue

    init(isSelected: Bool = false, accentColor: Color = .blue, @ViewBuilder content: () -> Content) {
        self.content = content()
        self.isSelected = isSelected
        self.accentColor = accentColor
    }

    var body: some View {
        content
            .background(
                RoundedRectangle(cornerRadius: 20)
                    .fill(.ultraThinMaterial)
                    .overlay(
                        RoundedRectangle(cornerRadius: 20)
                            .stroke(
                                isSelected ? accentColor : Color.white.opacity(0.1),
                                lineWidth: isSelected ? 2 : 1
                            )
                    )
                    .shadow(color: Color.black.opacity(0.2), radius: 10, x: 0, y: 5)
            )
    }
}

// MARK: - Gradient Icon Background

struct GradientIconBackground: View {
    let icon: String
    let colors: [Color]
    var size: CGFloat = 48

    var body: some View {
        ZStack {
            Circle()
                .fill(
                    LinearGradient(
                        colors: colors,
                        startPoint: .topLeading,
                        endPoint: .bottomTrailing
                    )
                )
                .frame(width: size, height: size)

            Image(systemName: icon)
                .font(.system(size: size * 0.45, weight: .semibold))
                .foregroundColor(.white)
        }
    }
}

// MARK: - Clean Button

struct CleanButton: View {
    let size: String
    let itemCount: Int
    let color: Color
    let isLoading: Bool
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            HStack(spacing: 8) {
                if isLoading {
                    ProgressView()
                        .scaleEffect(0.8)
                        .progressViewStyle(CircularProgressViewStyle(tint: .white))
                } else {
                    Image(systemName: "sparkles")
                        .font(.headline)
                }

                VStack(alignment: .leading, spacing: 0) {
                    Text("Clean \(size)")
                        .font(.headline)
                        .fontWeight(.semibold)
                    if itemCount > 0 {
                        Text("\(itemCount) items")
                            .font(.caption)
                            .opacity(0.8)
                    }
                }
            }
            .foregroundColor(.white)
            .padding(.horizontal, 20)
            .padding(.vertical, 12)
            .background(
                RoundedRectangle(cornerRadius: 12)
                    .fill(
                        LinearGradient(
                            colors: [color, color.opacity(0.8)],
                            startPoint: .topLeading,
                            endPoint: .bottomTrailing
                        )
                    )
            )
            .shadow(color: color.opacity(0.4), radius: 8, x: 0, y: 4)
        }
        .buttonStyle(.plain)
        .disabled(isLoading)
    }
}

// MARK: - Scan Button

struct ScanButton: View {
    let isScanning: Bool
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            HStack(spacing: 8) {
                if isScanning {
                    ProgressView()
                        .scaleEffect(0.8)
                        .progressViewStyle(CircularProgressViewStyle(tint: .white))
                } else {
                    Image(systemName: "magnifyingglass")
                        .font(.headline)
                }
                Text(isScanning ? "Scanning..." : "Scan All")
                    .font(.headline)
                    .fontWeight(.semibold)
            }
            .foregroundColor(.white)
            .padding(.horizontal, 24)
            .padding(.vertical, 12)
            .background(
                RoundedRectangle(cornerRadius: 12)
                    .fill(
                        LinearGradient(
                            colors: [.blue, .blue.opacity(0.8)],
                            startPoint: .topLeading,
                            endPoint: .bottomTrailing
                        )
                    )
            )
            .shadow(color: .blue.opacity(0.4), radius: 8, x: 0, y: 4)
        }
        .buttonStyle(.plain)
        .disabled(isScanning)
    }
}

// MARK: - View Mode Toggle

struct ViewModeToggle: View {
    @Binding var mode: ViewMode

    var body: some View {
        HStack(spacing: 4) {
            ForEach(ViewMode.allCases, id: \.self) { viewMode in
                Button {
                    withAnimation(.spring(response: 0.3)) {
                        mode = viewMode
                    }
                } label: {
                    HStack(spacing: 4) {
                        Image(systemName: viewMode.icon)
                            .font(.caption)
                        Text(viewMode.rawValue)
                            .font(.caption)
                    }
                    .foregroundColor(mode == viewMode ? .white : .secondary)
                    .padding(.horizontal, 12)
                    .padding(.vertical, 6)
                    .background(
                        RoundedRectangle(cornerRadius: 8)
                            .fill(mode == viewMode ? Color.blue : Color.clear)
                    )
                }
                .buttonStyle(.plain)
            }
        }
        .padding(4)
        .background(
            RoundedRectangle(cornerRadius: 10)
                .fill(.ultraThinMaterial)
        )
    }
}

// MARK: - Disk Usage Ring

struct DiskUsageRing: View {
    let usagePercent: Double
    let size: CGFloat

    var body: some View {
        ZStack {
            // Background ring
            Circle()
                .stroke(Color.secondary.opacity(0.2), lineWidth: size * 0.12)

            // Usage ring
            Circle()
                .trim(from: 0, to: usagePercent / 100)
                .stroke(
                    AngularGradient(
                        colors: [usageColor, usageColor.opacity(0.6)],
                        center: .center,
                        startAngle: .degrees(-90),
                        endAngle: .degrees(270)
                    ),
                    style: StrokeStyle(lineWidth: size * 0.12, lineCap: .round)
                )
                .rotationEffect(.degrees(-90))

            // Icon
            Image(systemName: "internaldrive.fill")
                .font(.system(size: size * 0.3))
                .foregroundColor(.blue)
        }
        .frame(width: size, height: size)
    }

    private var usageColor: Color {
        if usagePercent > 90 { return .red }
        if usagePercent > 75 { return .orange }
        return .green
    }
}

// MARK: - Preview

#Preview {
    ZStack {
        Color.black.opacity(0.8)
            .ignoresSafeArea()

        VStack(spacing: 20) {
            GlassCard {
                VStack {
                    GradientIconBackground(
                        icon: "folder.badge.gearshape",
                        colors: [.blue, .cyan]
                    )
                    Text("System Junk")
                        .font(.headline)
                        .foregroundColor(.white)
                }
                .padding(20)
            }

            CleanButton(
                size: "2.5 GB",
                itemCount: 42,
                color: .blue,
                isLoading: false,
                action: {}
            )

            ViewModeToggle(mode: .constant(.normie))

            DiskUsageRing(usagePercent: 75, size: 80)
        }
        .padding()
    }
}
