//
//  crystalclearApp.swift
//  crystalclear
//
//  Crystal Clear - A safe macOS system cleaner.
//

import SwiftUI

@main
struct CrystalClearApp: App {
    var body: some Scene {
        WindowGroup {
            ContentView()
                .frame(minWidth: 900, minHeight: 600)
        }
        .windowStyle(.hiddenTitleBar)
        .windowResizability(.contentSize)
    }
}
