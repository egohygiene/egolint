import Foundation

struct GreetingService {
    func formatGreeting(for name: String) -> String {
        let normalizedName = name.trimmingCharacters(
            in: .whitespacesAndNewlines
        )

        if normalizedName.isEmpty {
            return "Hello."
        }

        return "Hello, \(normalizedName)."
    }
}
