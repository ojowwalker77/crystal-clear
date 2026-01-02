//
//  DashboardView.swift
//  crystalclear
//
//  Main dashboard view with category cards.
//

import SwiftUI

struct DashboardView: View {
    @StateObject private var viewModel = DashboardViewModel()

    var body: some View {
        NavigationSplitView {
            // Sidebar with dashboard content
            ScrollView {
                VStack(spacing: 20) {
                    // Disk usage header
                    DiskUsageHeader(
                        diskInfo: viewModel.diskInfo,
                        potentialSavings: viewModel.formattedTotalSavings,
                        isScanning: viewModel.isScanningAny,
                        onScanAll: {
                            Task {
                                await viewModel.scanAll()
                            }
                        }
                    )

                    // Category cards grid
                    CategoryCardsGrid(viewModel: viewModel)
                }
                .padding()
            }
            .frame(minWidth: 500)
            .navigationTitle("Crystal Clear")
            .toolbar {
                ToolbarItemGroup(placement: .primaryAction) {
                    if !viewModel.selectedItems.isEmpty {
                        Text("\(viewModel.selectedItems.count) selected (\(viewModel.formattedSelectedSize))")
                            .font(.caption)
                            .foregroundColor(.secondary)

                        Button {
                            Task {
                                await viewModel.cleanSelectedItems()
                            }
                        } label: {
                            HStack {
                                if viewModel.isCleaning {
                                    ProgressView()
                                        .scaleEffect(0.7)
                                } else {
                                    Image(systemName: "trash")
                                }
                                Text(viewModel.isCleaning ? "Cleaning..." : "Clean")
                            }
                        }
                        .buttonStyle(.borderedProminent)
                        .disabled(viewModel.isCleaning)

                        Button {
                            viewModel.deselectAll()
                        } label: {
                            Text("Clear Selection")
                        }
                        .buttonStyle(.bordered)
                    }
                }
            }
        } detail: {
            // Detail view for selected category
            if let category = viewModel.selectedCategory {
                CategoryDetailContainer(category: category, viewModel: viewModel)
            } else {
                ContentUnavailableView {
                    Label("Select a Category", systemImage: "hand.tap")
                } description: {
                    Text("Click on a category card to see details and select items to clean.")
                }
            }
        }
        .alert("Operation Complete", isPresented: .init(
            get: { viewModel.lastResultMessage != nil },
            set: { if !$0 { viewModel.lastResultMessage = nil } }
        )) {
            Button("OK") {
                viewModel.lastResultMessage = nil
            }
        } message: {
            Text(viewModel.lastResultMessage ?? "")
        }
    }
}

/// Container for category detail views
struct CategoryDetailContainer: View {
    let category: DashboardCategory
    @ObservedObject var viewModel: DashboardViewModel

    var body: some View {
        VStack(spacing: 0) {
            switch category {
            case .systemJunk:
                SystemJunkDetailView(viewModel: viewModel)
            case .largeFiles:
                LargeFilesDetailView(viewModel: viewModel)
            case .duplicates:
                DuplicatesDetailView(viewModel: viewModel)
            case .downloads:
                DownloadsDetailView(viewModel: viewModel)
            case .applications:
                AppsDetailView(viewModel: viewModel)
            }
        }
        .frame(minWidth: 400)
    }
}

#Preview {
    DashboardView()
}
