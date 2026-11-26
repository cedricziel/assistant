import 'dart:async';
import 'dart:convert';

import 'package:assistant_ui/models.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_ui/material_ui.dart';
import 'package:url_launcher/url_launcher.dart';

import '../../shared/platform/adaptive_dialog.dart';
import 'mcp_app/mcp_app_bridge.dart';
import 'mcp_app/mcp_app_provider.dart';
import 'mcp_app/mcp_app_surface.dart';

const _initialHeight = 160.0;
const _minHeight = 48.0;
const _maxHeight = 800.0;

/// The MCP App of [toolCall], rendered in a sandbox below its chip and marked
/// as third-party content from its server. Renders nothing for a tool call
/// without an app.
class McpAppFrame extends StatelessWidget {
  const McpAppFrame({super.key, required this.toolCall, this.conversationId});

  final ToolCallRecord toolCall;

  /// The conversation the tool call belongs to; without it the app cannot
  /// call its server.
  final String? conversationId;

  @override
  Widget build(BuildContext context) {
    final app = toolCall.app;
    if (app == null) return const SizedBox.shrink();
    return _McpAppFrameBody(
      // A different call at the same position needs its own surface and
      // bridge, not this one's.
      key: ValueKey((toolCall.toolCallId, app.resourceUri)),
      app: app,
      toolCall: toolCall,
      conversationId: conversationId,
    );
  }
}

class _McpAppFrameBody extends ConsumerStatefulWidget {
  const _McpAppFrameBody({
    super.key,
    required this.app,
    required this.toolCall,
    required this.conversationId,
  });

  final McpApp app;
  final ToolCallRecord toolCall;
  final String? conversationId;

  @override
  ConsumerState<_McpAppFrameBody> createState() => _McpAppFrameState();
}

class _McpAppFrameState extends ConsumerState<_McpAppFrameBody> {
  late final McpAppSurface _surface;
  McpAppBridge? _bridge;
  double _height = _initialHeight;

  @override
  void initState() {
    super.initState();
    _surface = ref.read(mcpAppSurfaceFactoryProvider)(widget.app);
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _bridge ??= McpAppBridge(
      port: _surface.port,
      backend: _CurrentBackend(_backend),
      toolCall: widget.toolCall,
      hostContext: _hostContext(context),
      confirmToolCall: _confirmToolCall,
      confirmOpenLink: _confirmOpenLink,
      openLink: (url) => launchUrl(url, mode: LaunchMode.externalApplication),
      onSizeChanged: (height) {
        final clamped = height.clamp(_minHeight, _maxHeight);
        if (!mounted || clamped == _height) return;
        setState(() => _height = clamped);
      },
    );
  }

  @override
  void dispose() {
    final surface = _surface;
    unawaited(
      (_bridge?.close() ?? Future<void>.value()).whenComplete(surface.dispose),
    );
    super.dispose();
  }

  McpAppBackend _backend() {
    if (!mounted) return const _UnavailableBackend();
    final conversationId = widget.conversationId;
    final toolCallId = widget.toolCall.toolCallId;
    final backend = conversationId == null || toolCallId == null
        ? null
        : ref.read(
            mcpAppBackendProvider((
              conversationId: conversationId,
              toolCallId: toolCallId,
            )),
          );
    return backend ?? const _UnavailableBackend();
  }

  Future<bool> _confirmToolCall(String name, Map<String, Object?> arguments) {
    final details = arguments.isEmpty
        ? ''
        : '\n\n${const JsonEncoder.withIndent('  ').convert(arguments)}';
    return _confirm(
      title: 'Allow ${widget.app.server} to run "$name"?',
      content: 'The app asks to run a tool that may change data.$details',
      confirmLabel: 'Run',
    );
  }

  Future<bool> _confirmOpenLink(Uri url) => _confirm(
    title: 'Open link?',
    content: '${widget.app.server} wants to open $url',
    confirmLabel: 'Open',
  );

  Future<bool> _confirm({
    required String title,
    required String content,
    required String confirmLabel,
  }) async {
    if (!mounted) return false;
    return showAdaptiveConfirmDialog(
      context: context,
      title: title,
      content: content,
      confirmLabel: confirmLabel,
    );
  }

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final label = 'Third-party app from ${widget.app.server}';
    return Align(
      alignment: Alignment.centerLeft,
      child: Container(
        margin: const EdgeInsets.only(top: 4, bottom: 8),
        constraints: const BoxConstraints(maxWidth: 640),
        decoration: BoxDecoration(
          borderRadius: BorderRadius.circular(8),
          border: Border.all(color: colorScheme.outlineVariant),
        ),
        clipBehavior: Clip.antiAlias,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          mainAxisSize: MainAxisSize.min,
          children: [
            Semantics(
              header: true,
              child: Container(
                color: colorScheme.surfaceContainerHighest,
                padding: const EdgeInsets.symmetric(
                  horizontal: 10,
                  vertical: 4,
                ),
                child: Row(
                  children: [
                    Icon(
                      Icons.extension_outlined,
                      size: 14,
                      color: colorScheme.onSurfaceVariant,
                    ),
                    const SizedBox(width: 6),
                    Flexible(
                      child: Text(
                        label,
                        style: TextStyle(
                          fontSize: 11,
                          color: colorScheme.onSurfaceVariant,
                        ),
                        overflow: TextOverflow.ellipsis,
                      ),
                    ),
                  ],
                ),
              ),
            ),
            Semantics(
              label: label,
              container: true,
              child: SizedBox(height: _height, child: _surface.build(context)),
            ),
          ],
        ),
      ),
    );
  }
}

McpAppHostContext _hostContext(BuildContext context) => McpAppHostContext(
  theme: Theme.of(context).brightness == Brightness.dark ? 'dark' : 'light',
  locale: (Localizations.maybeLocaleOf(context) ?? const Locale('en'))
      .toLanguageTag(),
  timeZone: DateTime.now().timeZoneName,
  platform: kIsWeb
      ? 'web'
      : switch (defaultTargetPlatform) {
          TargetPlatform.iOS || TargetPlatform.android => 'mobile',
          _ => 'desktop',
        },
);

/// Refuses every call, for a tool call this client cannot route.
class _UnavailableBackend implements McpAppBackend {
  const _UnavailableBackend();

  static const _refusal = McpAppRefused(
    null,
    'This app cannot reach its server from here',
  );

  @override
  Future<Map<String, Object?>> callTool(
    String name,
    Map<String, Object?> arguments, {
    required bool confirmed,
  }) => Future.error(_refusal);

  @override
  Future<Map<String, Object?>> readResource(String uri) =>
      Future.error(_refusal);
}

/// Looks the backend up on every call, so a reconnect or a new API client
/// reaches an app that was rendered earlier.
class _CurrentBackend implements McpAppBackend {
  const _CurrentBackend(this._resolve);

  final McpAppBackend Function() _resolve;

  @override
  Future<Map<String, Object?>> callTool(
    String name,
    Map<String, Object?> arguments, {
    required bool confirmed,
  }) => _resolve().callTool(name, arguments, confirmed: confirmed);

  @override
  Future<Map<String, Object?>> readResource(String uri) =>
      _resolve().readResource(uri);
}
