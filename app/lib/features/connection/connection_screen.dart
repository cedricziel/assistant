import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../contexts/models/context_model.dart';
import '../contexts/providers/context_providers.dart';
import '../embedded_server/embedded_server_provider.dart';
import '../embedded_server/embedded_server_service.dart';
import 'connection_provider.dart';
import 'server_mode.dart';

/// Setup screen — shown when no active context exists and the embedded server
/// is not available (or is still starting up).
///
/// On macOS with the bundled binary the screen shows the embedded-server
/// startup progress.  In remote mode (or on web) it renders a URL + token
/// form that creates and activates a new [AssistantContext].
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
  bool _tokenVisible = false;
  bool _connecting = false;
  String? _error;

  @override
  void initState() {
    super.initState();
    _mode = EmbeddedServerService.isAvailable
        ? ServerMode.embedded
        : ServerMode.remote;

    // On web, honour `_token` and optional `_url` query parameters so that
    // automated flows (e2e tests, deep links) can connect without any user
    // interaction.  The form is pre-populated and auto-submitted after the
    // first frame.
    final autoToken = isWebPlatform ? Uri.base.queryParameters['_token'] : null;
    final autoUrl = isWebPlatform ? Uri.base.queryParameters['_url'] : null;

    _urlController = TextEditingController(
      text:
          autoUrl ??
          (isWebPlatform ? Uri.base.origin : 'http://127.0.0.1:8080'),
    );
    _tokenController = TextEditingController(text: autoToken ?? '');

    if (autoToken != null && autoToken.isNotEmpty) {
      WidgetsBinding.instance.addPostFrameCallback((_) => _connect());
    }
  }

  @override
  void dispose() {
    _urlController.dispose();
    _tokenController.dispose();
    super.dispose();
  }

  Future<void> _connect() async {
    if (!_formKey.currentState!.validate()) return;
    setState(() {
      _connecting = true;
      _error = null;
    });
    try {
      final url = _urlController.text.trim().replaceAll(RegExp(r'/$'), '');
      final token = _tokenController.text.trim();
      final ctx = AssistantContext.create(
        name: Uri.parse(url).host,
        serverUrl: url,
        authToken: token.isEmpty ? null : token,
      );
      await ref.read(contextsProvider.notifier).saveContext(ctx);
      await ref.read(activeContextProvider.notifier).activate(ctx);
      if (mounted) context.go('/chat');
    } catch (e) {
      setState(() => _error = e.toString());
    } finally {
      if (mounted) setState(() => _connecting = false);
    }
  }

  Future<void> _switchToRemote() async {
    await ref.read(embeddedServerProvider.notifier).stop();
    setState(() => _mode = ServerMode.remote);
  }

  void _switchToEmbedded() {
    setState(() => _mode = ServerMode.embedded);
    ref.invalidate(embeddedServerProvider);
  }

  @override
  Widget build(BuildContext context) {
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
                  const Text(
                    'Connect to Assistant',
                    style: TextStyle(fontSize: 24, fontWeight: FontWeight.bold),
                    textAlign: TextAlign.center,
                  ),
                  const SizedBox(height: 8),
                  Text(
                    'Choose how to connect to the backend.',
                    textAlign: TextAlign.center,
                    style: TextStyle(
                      color: Theme.of(context).colorScheme.onSurfaceVariant,
                    ),
                  ),
                  const SizedBox(height: 24),

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

                  if (_mode == ServerMode.embedded) ...[
                    _EmbeddedServerStatus(embeddedState: embeddedState),
                  ] else ...[
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
                            decoration: InputDecoration(
                              labelText: 'Authentication Token (optional)',
                              hintText: 'Paste your token here',
                              border: const OutlineInputBorder(),
                              prefixIcon: const Icon(Icons.key_outlined),
                              suffixIcon: IconButton(
                                icon: Icon(
                                  _tokenVisible
                                      ? Icons.visibility_off_outlined
                                      : Icons.visibility_outlined,
                                ),
                                onPressed: () => setState(
                                  () => _tokenVisible = !_tokenVisible,
                                ),
                              ),
                            ),
                            obscureText: !_tokenVisible,
                            autocorrect: false,
                          ),
                          if (_error != null) ...[
                            const SizedBox(height: 8),
                            Container(
                              padding: const EdgeInsets.symmetric(
                                horizontal: 12,
                                vertical: 8,
                              ),
                              decoration: BoxDecoration(
                                color: Theme.of(
                                  context,
                                ).colorScheme.errorContainer,
                                border: Border.all(
                                  color: Theme.of(context).colorScheme.error,
                                ),
                                borderRadius: BorderRadius.circular(8),
                              ),
                              child: Row(
                                children: [
                                  Icon(
                                    Icons.error_outline,
                                    color: Theme.of(
                                      context,
                                    ).colorScheme.onErrorContainer,
                                    size: 18,
                                  ),
                                  const SizedBox(width: 8),
                                  Expanded(
                                    child: Text(
                                      _error!,
                                      key: const Key('error_message'),
                                      style: TextStyle(
                                        color: Theme.of(
                                          context,
                                        ).colorScheme.onErrorContainer,
                                      ),
                                    ),
                                  ),
                                ],
                              ),
                            ),
                          ],
                          const SizedBox(height: 24),
                          FilledButton(
                            key: const Key('connect_button'),
                            onPressed: _connecting ? null : _connect,
                            child: _connecting
                                ? const SizedBox(
                                    height: 18,
                                    width: 18,
                                    child: CircularProgressIndicator(
                                      strokeWidth: 2,
                                    ),
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
      ),
    );
  }
}

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
        icon: Icon(
          Icons.error_outline,
          color: Theme.of(context).colorScheme.error,
        ),
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
          icon: Icon(
            Icons.error_outline,
            color: Theme.of(context).colorScheme.error,
          ),
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
    final colorScheme = Theme.of(context).colorScheme;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 20),
      decoration: BoxDecoration(
        color: isError
            ? colorScheme.errorContainer
            : colorScheme.surfaceContainerHighest,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(
          color: isError ? colorScheme.error : colorScheme.outline,
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
                color: isError ? colorScheme.onErrorContainer : null,
              ),
            ),
          ),
        ],
      ),
    );
  }
}
