import 'package:a2ui_flutter/a2ui_flutter.dart';
import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:flutter/foundation.dart' show listEquals;
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_ui/material_ui.dart';

import '../../../api/api_client.dart';
import '../../../api/capabilities_provider.dart';
import '../../connection/connection_provider.dart';
import 'a2ui_surface_view.dart';
import 'assistant_catalog.dart';
import 'attachment_image_loader.dart';
import 'link_opener.dart';

/// One live surface timeline entry: [SurfaceFrame] chrome around an
/// [A2uiSurfaceView] built from the surface's A2UI messages.
///
/// The catalog needs the authenticated HTTP client (attachment images) and the
/// org's trusted link hosts, so this is the app's I/O-backed adapter; the frame
/// itself stays pure. A surface whose messages do not render falls back to its
/// text, never to a broken tree.
class SurfaceEntry extends ConsumerStatefulWidget {
  const SurfaceEntry({
    super.key,
    required this.surface,
    this.entryState = EntryState.complete,
  });

  final SurfaceRecord surface;
  final EntryState entryState;

  @override
  ConsumerState<SurfaceEntry> createState() => _SurfaceEntryState();
}

class _SurfaceEntryState extends ConsumerState<SurfaceEntry> {
  CatalogImplementation? _catalog;
  ApiClient? _client;
  List<String> _hosts = const [];
  bool _failed = false;

  @override
  void didUpdateWidget(SurfaceEntry oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.surface.surfaceId != widget.surface.surfaceId ||
        !identical(oldWidget.surface.messages, widget.surface.messages)) {
      _failed = false;
    }
  }

  @override
  Widget build(BuildContext context) {
    final api = ref.watch(apiClientProvider);
    final hosts = ref.watch(uiLinkHostsProvider);
    if (!identical(api, _client) || !listEquals(hosts, _hosts)) {
      _client = api;
      _hosts = hosts;
      _catalog = api == null
          ? null
          : assistantCatalog(
              imageLoader: DioAttachmentImageLoader(api.dio),
              openUrl: confirmingLinkOpener(
                context: () => context,
                linkHosts: () => _hosts,
              ),
            );
    }

    final surface = widget.surface;
    return SurfaceFrame(
      title: surface.title,
      status: surface.status,
      answerable: surface.answerable,
      fallbackText: surface.fallbackText,
      hasError: _failed,
      awaitingInput: widget.entryState == EntryState.awaitingInput,
      content: _catalog == null || surface.messages.isEmpty
          ? null
          : A2uiSurfaceView(
              messages: surface.messages,
              catalog: _catalog!,
              onError: (_) {
                if (mounted && !_failed) setState(() => _failed = true);
              },
            ),
    );
  }
}
