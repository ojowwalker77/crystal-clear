//
//  AppIconCache.swift
//  crystalclear
//
//  Cache for application icons.
//

import AppKit
import SwiftUI

/// Singleton cache for application icons.
final class AppIconCache {
    static let shared = AppIconCache()

    private var cache: [String: NSImage] = [:]
    private let lock = NSLock()

    private init() {}

    /// Get the icon for an app at the given path.
    func icon(forAppAt path: String) -> NSImage {
        lock.lock()
        defer { lock.unlock() }

        if let cached = cache[path] {
            return cached
        }

        let icon = NSWorkspace.shared.icon(forFile: path)
        icon.size = NSSize(width: 32, height: 32)
        cache[path] = icon

        return icon
    }

    /// Clear the cache.
    func clearCache() {
        lock.lock()
        defer { lock.unlock() }
        cache.removeAll()
    }
}

/// SwiftUI view for displaying an app icon.
struct AppIconView: View {
    let appPath: String
    var size: CGFloat = 32

    @State private var icon: NSImage?

    var body: some View {
        Group {
            if let icon = icon {
                Image(nsImage: icon)
                    .resizable()
                    .aspectRatio(contentMode: .fit)
                    .frame(width: size, height: size)
            } else {
                RoundedRectangle(cornerRadius: 6)
                    .fill(Color.gray.opacity(0.3))
                    .frame(width: size, height: size)
            }
        }
        .onAppear {
            loadIcon()
        }
    }

    private func loadIcon() {
        DispatchQueue.global(qos: .userInitiated).async {
            let loadedIcon = AppIconCache.shared.icon(forAppAt: appPath)
            DispatchQueue.main.async {
                self.icon = loadedIcon
            }
        }
    }
}

#Preview {
    VStack(spacing: 10) {
        AppIconView(appPath: "/Applications/Safari.app")
        AppIconView(appPath: "/Applications/Xcode.app", size: 48)
    }
    .padding()
}
