import SwiftUI

/// Shared content that was extracted from the share sheet's `NSItemProvider`.
public struct SharedContent {
    public let fileData: Data?
    public let fileURL: URL?
    public let filename: String
    public let mimeType: String
    public let isURL: Bool
    public let urlString: String?

    public init(
        fileData: Data? = nil,
        fileURL: URL? = nil,
        filename: String = "attachment",
        mimeType: String = "application/octet-stream",
        isURL: Bool = false,
        urlString: String? = nil
    ) {
        self.fileData = fileData
        self.fileURL = fileURL
        self.filename = filename
        self.mimeType = mimeType
        self.isURL = isURL
        self.urlString = urlString
    }
}

/// The main SwiftUI view for the share extension, used on both iOS and macOS.
///
/// Displays a file preview, conversation picker, persona selector, optional
/// message field, and Send/Cancel buttons.
public struct ShareExtensionView: View {
    let items: [NSExtensionItem]
    let onDismiss: () -> Void

    @State private var conversations: [AssistantAPIClient.ConversationSummary] = []
    @State private var personas: [AssistantAPIClient.PersonaSummary] = []
    @State private var selectedConversationId: String? = nil
    @State private var selectedPersonaId: String? = nil
    @State private var messageText: String = ""
    @State private var isSending = false
    @State private var errorMessage: String?
    @State private var sharedContent: SharedContent?
    @State private var hasCredentials = false
    @State private var isLoading = true

    private let api = AssistantAPIClient.shared

    public init(items: [NSExtensionItem], onDismiss: @escaping () -> Void) {
        self.items = items
        self.onDismiss = onDismiss
    }

    public var body: some View {
        NavigationView {
            Group {
                if !hasCredentials {
                    noCredentialsView
                } else if isLoading {
                    ProgressView("Loading...")
                        .frame(maxWidth: .infinity, maxHeight: .infinity)
                } else {
                    formContent
                }
            }
            .navigationTitle("Share with Assistant")
            #if os(iOS)
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") { onDismiss() }
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button("Send") { Task { await send() } }
                        .disabled(!canSend)
                }
            }
            #else
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") { onDismiss() }
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button("Send") { Task { await send() } }
                        .disabled(!canSend)
                }
            }
            #endif
        }
        .task { await loadData() }
    }

    // MARK: - Subviews

    private var noCredentialsView: some View {
        VStack(spacing: 16) {
            Image(systemName: "key.slash")
                .font(.system(size: 48))
                .foregroundColor(.secondary)
            Text("Not Connected")
                .font(.headline)
            Text("Open the Assistant app and connect to your server first.")
                .font(.subheadline)
                .foregroundColor(.secondary)
                .multilineTextAlignment(.center)
                .padding(.horizontal)
            Button("Done") { onDismiss() }
                .buttonStyle(.borderedProminent)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private var formContent: some View {
        Form {
            // File preview
            if let content = sharedContent {
                Section("Sharing") {
                    HStack {
                        fileIcon(for: content)
                            .font(.title2)
                            .foregroundColor(.accentColor)
                        VStack(alignment: .leading) {
                            Text(content.filename)
                                .font(.body)
                                .lineLimit(1)
                            Text(content.mimeType)
                                .font(.caption)
                                .foregroundColor(.secondary)
                        }
                    }
                }
            }

            // Conversation picker
            Section("Conversation") {
                Picker("Conversation", selection: $selectedConversationId) {
                    Text("New Conversation").tag(nil as String?)
                    ForEach(conversations) { conv in
                        Text(conv.title ?? "Untitled")
                            .tag(conv.id as String?)
                    }
                }
            }

            // Persona selector
            if !personas.isEmpty {
                Section("Persona") {
                    Picker("Persona", selection: $selectedPersonaId) {
                        Text("Default").tag(nil as String?)
                        ForEach(personas) { persona in
                            Text(persona.name).tag(persona.id as String?)
                        }
                    }
                }
            }

            // Message field
            Section("Message") {
                TextField("Add a message (optional)", text: $messageText, axis: .vertical)
                    .lineLimit(3...6)
            }

            // Error
            if let errorMessage {
                Section {
                    Label(errorMessage, systemImage: "exclamationmark.triangle")
                        .foregroundColor(.red)
                }
            }

            // Progress
            if isSending {
                Section {
                    HStack {
                        ProgressView()
                        Text("Sending...")
                            .foregroundColor(.secondary)
                    }
                }
            }
        }
    }

    private func fileIcon(for content: SharedContent) -> Image {
        if content.isURL {
            return Image(systemName: "link")
        }
        let mime = content.mimeType
        if mime.hasPrefix("image/") {
            return Image(systemName: "photo")
        } else if mime == "application/pdf" {
            return Image(systemName: "doc.richtext")
        } else if mime.hasPrefix("text/") || mime == "application/json" {
            return Image(systemName: "doc.text")
        }
        return Image(systemName: "doc")
    }

    // MARK: - State

    private var canSend: Bool {
        guard hasCredentials, !isSending else { return false }
        guard sharedContent != nil else { return false }
        return true
    }

    // MARK: - Data loading

    private func loadData() async {
        // Check credentials.
        let keychain = KeychainHelper(
            service: "com.cedricziel.assistant",
            sharedAccessGroup: "\(KeychainHelper.teamPrefix)com.cedricziel.assistant.shared"
        )
        hasCredentials = keychain.hasCredentials

        guard hasCredentials else {
            isLoading = false
            return
        }

        // Extract shared content from NSItemProviders.
        sharedContent = await extractContent(from: items)
        if sharedContent == nil {
            errorMessage = "No supported content found in the shared items."
        }

        // Fetch conversations and personas in parallel.
        async let convos = api.listConversations()
        async let personaList = api.listPersonas()
        do {
            conversations = try await convos
            personas = try await personaList
        } catch {
            errorMessage = error.localizedDescription
        }

        isLoading = false
    }

    // MARK: - Content extraction

    private func extractContent(from items: [NSExtensionItem]) async -> SharedContent? {
        for item in items {
            guard let providers = item.attachments else { continue }

            for provider in providers {
                // Check for URL first.
                if provider.hasItemConformingToTypeIdentifier("public.url") {
                    if let content = await extractURL(from: provider) {
                        return content
                    }
                }

                // Check for file data.
                let fileTypes = [
                    "public.image",
                    "com.adobe.pdf",
                    "public.plain-text",
                    "public.json",
                    "public.comma-separated-values-text",
                    "net.daringfireball.markdown",
                    "public.data",
                ]
                for type in fileTypes {
                    if provider.hasItemConformingToTypeIdentifier(type) {
                        if let content = await extractFile(from: provider, type: type) {
                            return content
                        }
                    }
                }
            }
        }
        return nil
    }

    private func extractURL(from provider: NSItemProvider) async -> SharedContent? {
        do {
            let item = try await provider.loadItem(forTypeIdentifier: "public.url")
            if let url = item as? URL {
                return SharedContent(
                    filename: url.absoluteString,
                    mimeType: "text/plain",
                    isURL: true,
                    urlString: url.absoluteString
                )
            }
        } catch {
            await MainActor.run {
                self.errorMessage = "Could not read shared URL: \(error.localizedDescription)"
            }
        }
        return nil
    }

    private func extractFile(from provider: NSItemProvider, type: String) async -> SharedContent? {
        do {
            let item = try await provider.loadItem(forTypeIdentifier: type)

            if let url = item as? URL {
                // Acquire security-scoped access for sandboxed share extensions.
                let didStart = url.startAccessingSecurityScopedResource()
                defer { if didStart { url.stopAccessingSecurityScopedResource() } }

                let data = try Data(contentsOf: url)
                let filename = url.lastPathComponent
                let mime = mimeType(for: filename, uti: type)

                // Handle HEIC → PNG conversion.
                if mime == "image/heic" || mime == "image/heif" {
                    if let converted = convertHEICToPNG(data: data, filename: filename) {
                        return converted
                    }
                }

                return SharedContent(
                    fileData: data, filename: filename, mimeType: mime
                )
            }

            if let data = item as? Data {
                let filename = provider.suggestedName ?? "attachment"
                let mime = mimeType(for: filename, uti: type)
                return SharedContent(fileData: data, filename: filename, mimeType: mime)
            }
        } catch {
            await MainActor.run {
                self.errorMessage = "Could not read shared file: \(error.localizedDescription)"
            }
        }
        return nil
    }

    /// Converts HEIC/HEIF image data to PNG, returning nil if conversion fails.
    private func convertHEICToPNG(data: Data, filename: String) -> SharedContent? {
        let pngName = filename.replacingOccurrences(
            of: "\\.(heic|heif)$",
            with: ".png",
            options: [.regularExpression, .caseInsensitive]
        )
        #if canImport(UIKit)
        if let uiImage = UIImage(data: data),
           let pngData = uiImage.pngData() {
            return SharedContent(fileData: pngData, filename: pngName, mimeType: "image/png")
        }
        #elseif canImport(AppKit)
        if let nsImage = NSImage(data: data),
           let tiffData = nsImage.tiffRepresentation,
           let bitmap = NSBitmapImageRep(data: tiffData),
           let pngData = bitmap.representation(using: .png, properties: [:]) {
            return SharedContent(fileData: pngData, filename: pngName, mimeType: "image/png")
        }
        #endif
        return nil
    }

    private func mimeType(for filename: String, uti: String) -> String {
        let ext = (filename as NSString).pathExtension.lowercased()
        switch ext {
        case "png": return "image/png"
        case "jpg", "jpeg": return "image/jpeg"
        case "gif": return "image/gif"
        case "webp": return "image/webp"
        case "heic", "heif": return "image/heic"
        case "pdf": return "application/pdf"
        case "txt": return "text/plain"
        case "md", "markdown": return "text/markdown"
        case "csv": return "text/csv"
        case "json": return "application/json"
        default:
            // Fallback based on UTI.
            switch uti {
            case "public.plain-text": return "text/plain"
            case "com.adobe.pdf": return "application/pdf"
            case "public.json": return "application/json"
            case "public.comma-separated-values-text": return "text/csv"
            case "net.daringfireball.markdown": return "text/markdown"
            case "public.image": return "application/octet-stream"
            default: return "application/octet-stream"
            }
        }
    }

    // MARK: - Send flow

    private func send() async {
        guard let content = sharedContent else { return }

        isSending = true
        errorMessage = nil

        do {
            // 1. Switch persona if different from default.
            if let personaId = selectedPersonaId {
                try await api.switchPersona(id: personaId)
            }

            // 2. Create or use existing conversation.
            let conversationId: String
            if let existingId = selectedConversationId {
                conversationId = existingId
            } else {
                let conv = try await api.createConversation(
                    title: content.isURL
                        ? content.urlString
                        : content.filename
                )
                conversationId = conv.id
            }

            // 3. Build the message text.
            var message = messageText.trimmingCharacters(in: .whitespacesAndNewlines)

            if content.isURL, let urlString = content.urlString {
                // URLs are sent as message text.
                let urlMessage = message.isEmpty ? urlString : "\(message)\n\n\(urlString)"
                try await api.sendMessage(
                    conversationId: conversationId,
                    message: urlMessage
                )
            } else if let fileData = content.fileData {
                // 4. Upload attachment.
                let attachment = try await api.uploadAttachment(
                    conversationId: conversationId,
                    fileData: fileData,
                    filename: content.filename,
                    mimeType: content.mimeType
                )

                // 5. Send message with attachment.
                if message.isEmpty {
                    message = "Shared file: \(content.filename)"
                }
                try await api.sendMessage(
                    conversationId: conversationId,
                    message: message,
                    attachmentIds: [attachment.id]
                )
            }

            onDismiss()
        } catch {
            errorMessage = error.localizedDescription
            isSending = false
        }
    }
}

#if canImport(UIKit)
import UIKit
#elseif canImport(AppKit)
import AppKit
#endif
