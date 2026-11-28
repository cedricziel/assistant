import 'package:a2ui_flutter/a2ui_flutter.dart';
import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:flutter/foundation.dart' show listEquals;
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_ui/material_ui.dart';

import '../../../api/api_client.dart';
import '../../../api/capabilities_provider.dart';
import '../../../api/models/surface_action.dart';
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
    this.pending = false,
    this.onAction,
  });

  final SurfaceRecord surface;
  final EntryState entryState;

  /// The surface's action request is queued or in flight.
  final bool pending;

  /// Called with a Button action (and its data model when the surface sends
  /// one). The host routes it to the interaction channel or the post-turn
  /// route; the frame itself sends nothing.
  final void Function(SurfaceRecord surface, SurfaceActionSubmission action)?
  onAction;

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
      pending: widget.pending,
      awaitingInput: widget.entryState == EntryState.awaitingInput,
      content: _catalog == null || surface.messages.isEmpty
          ? null
          : A2uiSurfaceView(
              messages: surface.messages,
              catalog: _catalog!,
              onAction: widget.onAction == null ? null : _submit,
              onError: (_) {
                if (mounted && !_failed) setState(() => _failed = true);
              },
            ),
    );
  }

  void _submit(SurfaceAction action) {
    widget.onAction?.call(
      widget.surface,
      SurfaceActionSubmission(
        surfaceId: widget.surface.surfaceId,
        name: action.action.name,
        sourceComponentId: action.action.sourceComponentId,
        context: Map<String, Object?>.from(action.action.context),
        dataModel: action.dataModel,
      ),
    );
  }
}
