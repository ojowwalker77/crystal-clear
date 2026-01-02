//
//  ConfirmationSheet.swift
//  crystalclear
//
//  Confirmation dialogs for dangerous operations.
//

import SwiftUI

struct CleanConfirmationSheet: View {
    let itemCount: Int
    let totalSize: String
    let onConfirm: () -> Void
    let onCancel: () -> Void

    var body: some View {
        VStack(spacing: 20) {
            Image(systemName: "trash")
                .font(.system(size: 48))
                .foregroundColor(.orange)

            Text("Move to Trash?")
                .font(.title2)
                .fontWeight(.bold)

            Text("This will move \(itemCount) item\(itemCount == 1 ? "" : "s") (\(totalSize)) to the Trash. You can restore them from the Trash if needed.")
                .multilineTextAlignment(.center)
                .foregroundColor(.secondary)

            HStack(spacing: 16) {
                Button("Cancel") {
                    onCancel()
                }
                .keyboardShortcut(.escape)

                Button("Move to Trash") {
                    onConfirm()
                }
                .keyboardShortcut(.return)
                .buttonStyle(.borderedProminent)
            }
        }
        .padding(24)
        .frame(width: 400)
    }
}

struct UninstallConfirmationSheet: View {
    let appCount: Int
    let includeLeftovers: Bool
    let onConfirm: (Bool) -> Void
    let onCancel: () -> Void

    @State private var cleanLeftovers: Bool

    init(appCount: Int, includeLeftovers: Bool, onConfirm: @escaping (Bool) -> Void, onCancel: @escaping () -> Void) {
        self.appCount = appCount
        self.includeLeftovers = includeLeftovers
        self.onConfirm = onConfirm
        self.onCancel = onCancel
        self._cleanLeftovers = State(initialValue: includeLeftovers)
    }

    var body: some View {
        VStack(spacing: 20) {
            Image(systemName: "trash.square")
                .font(.system(size: 48))
                .foregroundColor(.red)

            Text("Uninstall \(appCount) App\(appCount == 1 ? "" : "s")?")
                .font(.title2)
                .fontWeight(.bold)

            Text("The app\(appCount == 1 ? "" : "s") will be moved to Trash along with any associated files.")
                .multilineTextAlignment(.center)
                .foregroundColor(.secondary)

            Toggle("Also remove leftover files", isOn: $cleanLeftovers)
                .toggleStyle(.checkbox)

            HStack(spacing: 16) {
                Button("Cancel") {
                    onCancel()
                }
                .keyboardShortcut(.escape)

                Button("Uninstall") {
                    onConfirm(cleanLeftovers)
                }
                .keyboardShortcut(.return)
                .buttonStyle(.borderedProminent)
                .tint(.red)
            }
        }
        .padding(24)
        .frame(width: 400)
    }
}

struct ResultAlert: View {
    let message: String
    let onDismiss: () -> Void

    var body: some View {
        VStack(spacing: 16) {
            Image(systemName: "checkmark.circle.fill")
                .font(.system(size: 48))
                .foregroundColor(.green)

            Text("Complete")
                .font(.title2)
                .fontWeight(.bold)

            Text(message)
                .multilineTextAlignment(.center)
                .foregroundColor(.secondary)

            Button("OK") {
                onDismiss()
            }
            .keyboardShortcut(.return)
            .buttonStyle(.borderedProminent)
        }
        .padding(24)
        .frame(width: 350)
    }
}

#Preview {
    CleanConfirmationSheet(
        itemCount: 5,
        totalSize: "1.2 GB",
        onConfirm: {},
        onCancel: {}
    )
}
