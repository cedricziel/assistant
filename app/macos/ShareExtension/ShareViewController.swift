import AppKit
import SwiftUI
import AssistantIntents

/// Entry point for the macOS share extension.
///
/// Hosts the shared SwiftUI `ShareExtensionView` inside an `NSHostingController`.
class ShareViewController: NSViewController {
    override func loadView() {
        view = NSView(frame: NSRect(x: 0, y: 0, width: 400, height: 500))
    }

    override func viewDidLoad() {
        super.viewDidLoad()

        let items = extensionContext?.inputItems as? [NSExtensionItem] ?? []
        let hostingView = NSHostingView(
            rootView: ShareExtensionView(
                items: items,
                onDismiss: { [weak self] in
                    self?.extensionContext?.completeRequest(returningItems: nil)
                }
            )
        )

        hostingView.translatesAutoresizingMaskIntoConstraints = false
        view.addSubview(hostingView)
        NSLayoutConstraint.activate([
            hostingView.topAnchor.constraint(equalTo: view.topAnchor),
            hostingView.bottomAnchor.constraint(equalTo: view.bottomAnchor),
            hostingView.leadingAnchor.constraint(equalTo: view.leadingAnchor),
            hostingView.trailingAnchor.constraint(equalTo: view.trailingAnchor),
        ])
    }
}
