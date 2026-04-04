import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'connection_provider.dart';

/// Setup screen shown when no server profile is saved.
///
/// - **Web**: shows token field only (server URL auto-detected from page origin).
/// - **macOS**: shows both server URL and token fields.
class ConnectionScreen extends ConsumerStatefulWidget {
  const ConnectionScreen({super.key});

  @override
  ConsumerState<ConnectionScreen> createState() => _ConnectionScreenState();
}

class _ConnectionScreenState extends ConsumerState<ConnectionScreen> {
  final _formKey = GlobalKey<FormState>();
  late final TextEditingController _urlController;
  late final TextEditingController _tokenController;

  @override
  void initState() {
    super.initState();
    // Default URL for macOS — user may change it.
    _urlController = TextEditingController(text: 'http://127.0.0.1:8080');
    _tokenController = TextEditingController();
  }

  @override
  void dispose() {
    _urlController.dispose();
    _tokenController.dispose();
    super.dispose();
  }

  String get _serverUrl {
    if (isWebPlatform) {
      // Auto-detect from the browser origin.
      return Uri.base.origin;
    }
    return _urlController.text.trim();
  }

  Future<void> _connect() async {
    if (!_formKey.currentState!.validate()) return;

    await ref.read(serverProfileProvider.notifier).connect(
          _serverUrl,
          _tokenController.text.trim(),
        );
  }

  @override
  Widget build(BuildContext context) {
    final connectionState =
        ref.watch(serverProfileProvider).valueOrNull ?? const ServerConnectionState();
    final isConnecting = connectionState.isConnecting;
    final errorMessage = connectionState.error;

    return Scaffold(
      body: Center(
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 400),
          child: Padding(
            padding: const EdgeInsets.all(32),
            child: Form(
              key: _formKey,
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  // Title
                  const Text(
                    'Connect to Assistant',
                    style: TextStyle(
                      fontSize: 24,
                      fontWeight: FontWeight.bold,
                    ),
                    textAlign: TextAlign.center,
                  ),
                  const SizedBox(height: 8),
                  const Text(
                    'Enter your server credentials to get started.',
                    textAlign: TextAlign.center,
                    style: TextStyle(color: Colors.black54),
                  ),
                  const SizedBox(height: 32),

                  // Server URL field — shown on macOS only, hidden on web.
                  if (!isWebPlatform) ...[
                    TextFormField(
                      key: const Key('server_url_field'),
                      controller: _urlController,
                      decoration: const InputDecoration(
                        labelText: 'Server URL',
                        hintText: 'http://127.0.0.1:8080',
                        border: OutlineInputBorder(),
                        prefixIcon: Icon(Icons.dns_outlined),
                      ),
                      keyboardType: TextInputType.url,
                      autocorrect: false,
                      validator: (value) {
                        if (value == null || value.trim().isEmpty) {
                          return 'Server URL is required';
                        }
                        final uri = Uri.tryParse(value.trim());
                        if (uri == null ||
                            (!uri.scheme.startsWith('http'))) {
                          return 'Enter a valid http:// or https:// URL';
                        }
                        return null;
                      },
                    ),
                    const SizedBox(height: 16),
                  ],

                  // Token field — always shown.
                  TextFormField(
                    key: const Key('token_field'),
                    controller: _tokenController,
                    decoration: const InputDecoration(
                      labelText: 'Authentication Token',
                      hintText: 'Paste your token here',
                      border: OutlineInputBorder(),
                      prefixIcon: Icon(Icons.key_outlined),
                    ),
                    obscureText: true,
                    autocorrect: false,
                    validator: (value) {
                      if (value == null || value.trim().isEmpty) {
                        return 'Token is required';
                      }
                      return null;
                    },
                  ),
                  const SizedBox(height: 8),

                  // Error banner.
                  if (errorMessage != null) ...[
                    const SizedBox(height: 8),
                    Container(
                      padding: const EdgeInsets.symmetric(
                        horizontal: 12,
                        vertical: 8,
                      ),
                      decoration: BoxDecoration(
                        color: Colors.red.shade50,
                        border: Border.all(color: Colors.red.shade300),
                        borderRadius: BorderRadius.circular(8),
                      ),
                      child: Row(
                        children: [
                          Icon(
                            Icons.error_outline,
                            color: Colors.red.shade700,
                            size: 18,
                          ),
                          const SizedBox(width: 8),
                          Expanded(
                            child: Text(
                              errorMessage,
                              key: const Key('error_message'),
                              style: TextStyle(color: Colors.red.shade700),
                            ),
                          ),
                        ],
                      ),
                    ),
                  ],

                  const SizedBox(height: 24),

                  // Connect button.
                  FilledButton(
                    key: const Key('connect_button'),
                    onPressed: isConnecting ? null : _connect,
                    child: isConnecting
                        ? const SizedBox(
                            height: 18,
                            width: 18,
                            child: CircularProgressIndicator(strokeWidth: 2),
                          )
                        : const Text('Connect'),
                  ),

                  // Web hint: show auto-detected URL.
                  if (isWebPlatform) ...[
                    const SizedBox(height: 12),
                    Text(
                      'Connecting to: ${Uri.base.origin}',
                      textAlign: TextAlign.center,
                      style: const TextStyle(
                        fontSize: 12,
                        color: Colors.black45,
                      ),
                    ),
                  ],
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}
