//
//  CategoryCardsGrid.swift
//  crystalclear
//
//  Grid layout for category cards.
//

import SwiftUI

struct CategoryCardsGrid: View {
    @ObservedObject var viewModel: DashboardViewModel

    private let columns = [
        GridItem(.adaptive(minimum: 200, maximum: 280), spacing: 16)
    ]

    var body: some View {
        LazyVGrid(columns: columns, spacing: 16) {
            ForEach(DashboardCategory.allCases) { category in
                CategoryCardView(
                    category: category,
                    state: viewModel.categoryStats[category]?.state ?? .notScanned,
                    isSelected: viewModel.selectedCategory == category,
                    onTap: {
                        withAnimation(.spring(response: 0.3)) {
                            if viewModel.selectedCategory == category {
                                viewModel.selectedCategory = nil
                            } else {
                                viewModel.selectedCategory = category
                            }
                        }
                    },
                    onScan: {
                        Task {
                            await viewModel.scanCategory(category)
                        }
                    }
                )
            }
        }
        .padding(.horizontal, 4)
    }
}

#Preview {
    CategoryCardsGrid(viewModel: DashboardViewModel())
        .padding()
}
