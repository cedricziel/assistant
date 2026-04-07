import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../embedded_server/embedded_server_provider.dart';
import '../embedded_server/embedded_server_service.dart';
import 'connection_provider.dart';
import 'server_mode.dart';

/// Setup screen shown when no server profile is saved.
///
/// - **macOS with bundled binary**: shows an "Embedded (local)" default mode
///   that auto-starts the server; also allows switching to remote.
/// - **Web**: shows token field only (server URL auto-detected from page origin).
/// - **Other platforms / no bundle**: shows URL + token fields.
class ConnectionScreen extends ConsumerStatefulWidget {
  const ConnectionScreen({super.key});

  @override
  ConsumerState<ConnectionScreen> createState() => _ConnectionScreenState();
}

class _ConnectionScreenState extends ConsumerState<ConnectionScreen> {
  final _formKey = GlobalKey<FormState>();
  late final TextEditingController _urlController;
  late final TextEditingController _tokenController;
  late ServerMode _mode;

  @override
  void initState() {
    super.initState();
    _mode = EmbeddedServerService.isAvailable
        ? ServerMode.embedded
        : ServerMode.remote;

    _urlController = TextEditingController(
      text: isWebPlatform ? Uri.base.origin : 'http://127.0.0.1:8080',
    );
    _tokenController = TextEditingController();

    // Auto-connect when a `_token` query parameter is present in the URL.
    // Used by Playwright E2E tests to avoid interacting with the Flutter canvas.
    if (kIsWeb) {
      final autoToken = Uri.base.queryParameters['_token'];
      if (autoToken != null && autoToken.isNotEmpty) {
        WidgetsBinding.instance.addPostFrameCallback((_) {
          ref
              .read(serverProfileProvider.notifier)
              .connect(Uri.base.origin, autoToken);
        });
      }
    }
  }

  @override
  void dispose() {
    _urlController.dispose();
    _tokenController.dispose();
    super.dispose();
  }

  String get _serverUrl => _urlController.text.trim();

  Future<void> _connect() async {
    if (!_formKey.currentState!.validate()) return;

    await ref.read(serverProfileProvider.notifier).connect(
          _serverUrl,
          _tokenController.text.trim(),
        );
  }

  /// Switch to remote mode, stopping any running embedded server first.
  Future<void> _switchToRemote() async {
    await ref.read(embeddedServerProvider.notifier).stop();
    setState(() => _mode = ServerMode.remote);
  }

  void _switchToEmbedded() {
    setState(() => _mode = ServerMode.embedded);
    // Re-initialise the provider so it re-evaluates isAvailable and starts.
    ref.invalidate(embeddedServerProvider);
  }

  @override
  Widget build(BuildContext context) {
    final connectionState =
        ref.watch(serverProfileProvider).value ?? const ServerConnectionState();
    final isConnecting = connectionState.isConnecting;
    final errorMessage = connectionState.error;

    // Watch embedded server state only in embedded mode to avoid unnecessary
    // rebuilds when the user has switched to remote mode.
    final embeddedState = _mode == ServerMode.embedded
        ? ref.watch(embeddedServerProvider)
        : const AsyncData<EmbeddedServerState>(EmbeddedServerStopped());

    return Scaffold(
      body: SafeArea(
        child: Center(
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 400),
          child: Padding(
            padding: const EdgeInsets.all(32),
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
                  'Choose how to connect to the backend.',
                  textAlign: TextAlign.center,
                  style: TextStyle(color: Colors.black54),
                ),
                const SizedBox(height: 24),

                // Mode toggle — only shown when embedded binary is available.
                if (EmbeddedServerService.isAvailable) ...[
                  SegmentedButton<ServerMode>(
                    key: const Key('server_mode_toggle'),
                    segments: const [
                      ButtonSegment(
                        value: ServerMode.embedded,
                        label: Text('Embedded (local)'),
                        icon: Icon(Icons.computer_outlined),
                      ),
                      ButtonSegment(
                        value: ServerMode.remote,
                        label: Text('Remote server'),
                        icon: Icon(Icons.cloud_outlined),
                      ),
                    ],
                    selected: {_mode},
                    onSelectionChanged: (selection) {
                      final next = selection.first;
                      if (next == ServerMode.remote) {
                        _switchToRemote();
                      } else {
                        _switchToEmbedded();
                      }
                    },
                  ),
                  const SizedBox(height: 24),
                ],

                // Embedded mode — show server startup progress.
                if (_mode == ServerMode.embedded) ...[
                  _EmbeddedServerStatus(embeddedState: embeddedState),
                ] else ...[
                  // Remote mode — URL + token form.
                  Form(
                    key: _formKey,
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.stretch,
                      children: [
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
                                (uri.scheme != 'http' &&
                                    uri.scheme != 'https') ||
                                !uri.hasAuthority) {
                              return 'Enter a valid http:// or https:// URL';
                            }
                            return null;
                          },
                        ),
                        const SizedBox(height: 16),
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
                                    style:
                                        TextStyle(color: Colors.red.shade700),
                                  ),
                                ),
                              ],
                            ),
                          ),
                        ],

                        const SizedBox(height: 24),
                        FilledButton(
                          key: const Key('connect_button'),
                          onPressed: isConnecting ? null : _connect,
                          child: isConnecting
                              ? const SizedBox(
                                  height: 18,
                                  width: 18,
                                  child: CircularProgressIndicator(
                                      strokeWidth: 2),
                                )
                              : const Text('Connect'),
                        ),
                      ],
                    ),
                  ),
                ],
              ],
            ),
          ),
        ),
      ),
      ),  // SafeArea
    );
  }
}

/// Widget that displays the current state of the embedded server startup.
class _EmbeddedServerStatus extends StatelessWidget {
  const _EmbeddedServerStatus({required this.embeddedState});

  final AsyncValue<EmbeddedServerState> embeddedState;

  @override
  Widget build(BuildContext context) {
    return embeddedState.when(
      loading: () => const _StatusCard(
        key: Key('embedded_loading'),
        icon: CircularProgressIndicator(strokeWidth: 2),
        message: 'Starting local server…',
      ),
      error: (e, _) => _StatusCard(
        key: const Key('embedded_error'),
        icon: const Icon(Icons.error_outline, color: Colors.red),
        message: e.toString(),
        isError: true,
      ),
      data: (serverState) => switch (serverState) {
        EmbeddedServerStarting() => const _StatusCard(
            key: Key('embedded_starting'),
            icon: CircularProgressIndicator(strokeWidth: 2),
            message: 'Starting local server…',
          ),
        EmbeddedServerReady(baseUrl: final url) => _StatusCard(
            key: const Key('embedded_ready'),
            icon: const Icon(Icons.check_circle_outline, color: Colors.green),
            message: 'Server ready at $url — connecting…',
          ),
        EmbeddedServerError(message: final msg) => _StatusCard(
            key: const Key('embedded_error_state'),
            icon: const Icon(Icons.error_outline, color: Colors.red),
            message: msg,
            isError: true,
          ),
        EmbeddedServerStopped() => const _StatusCard(
            key: Key('embedded_stopped'),
            icon: Icon(Icons.stop_circle_outlined),
            message: 'Server stopped.',
          ),
      },
    );
  }
}

class _StatusCard extends StatelessWidget {
  const _StatusCard({
    super.key,
    required this.icon,
    required this.message,
    this.isError = false,
  });

  final Widget icon;
  final String message;
  final bool isError;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 20),
      decoration: BoxDecoration(
        color: isError ? Colors.red.shade50 : Colors.grey.shade100,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(
          color: isError ? Colors.red.shade200 : Colors.grey.shade300,
        ),
      ),
      child: Row(
        children: [
          SizedBox(width: 24, height: 24, child: Center(child: icon)),
          const SizedBox(width: 12),
          Expanded(
            child: Text(
              message,
              style: TextStyle(
                color: isError ? Colors.red.shade700 : null,
              ),
            ),
          ),
        ],
      ),
    );
  }
}
