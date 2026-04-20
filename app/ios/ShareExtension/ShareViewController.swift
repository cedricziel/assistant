import SwiftUI
import UIKit

/// Entry point for the iOS share extension.
///
/// Hosts the SwiftUI `ShareExtensionView` inside a `UIHostingController`.
class ShareViewController: UIViewController {
    override func viewDidLoad() {
        super.viewDidLoad()

        let items = extensionContext?.inputItems as? [NSExtensionItem] ?? []
        let hostingController = UIHostingController(
            rootView: ShareExtensionView(
                items: items,
                onDismiss: { [weak self] in
                    self?.extensionContext?.completeRequest(returningItems: nil)
                }
            )
        )

        addChild(hostingController)
        view.addSubview(hostingController.view)
        hostingController.view.translatesAutoresizingMaskIntoConstraints = false
        NSLayoutConstraint.activate([
            hostingController.view.topAnchor.constraint(equalTo: view.topAnchor),
            hostingController.view.bottomAnchor.constraint(equalTo: view.bottomAnchor),
            hostingController.view.leadingAnchor.constraint(equalTo: view.leadingAnchor),
            hostingController.view.trailingAnchor.constraint(equalTo: view.trailingAnchor),
        ])
        hostingController.didMove(toParent: self)
    }
}
