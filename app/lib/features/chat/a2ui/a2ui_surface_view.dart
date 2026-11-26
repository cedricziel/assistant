import 'dart:convert';

import 'package:a2ui_core/a2ui_core.dart';
import 'package:a2ui_flutter/a2ui_flutter.dart';
// ignore: deprecated_member_use
import 'package:flutter/widgets.dart';

import 'surface_theme.dart';

/// A user action on a surface: the A2UI action, plus the surface's data model
/// when the surface was created with `sendDataModel`.
class SurfaceAction {
  const SurfaceAction(this.action, {this.dataModel});

  final A2uiRendererAction action;
  final Map<String, Object?>? dataModel;
}

/// Renders one A2UI surface from its protocol [messages] (a `createSurface`,
/// then `updateComponents` and `updateDataModel`).
///
/// This is the app's one entry point to `a2ui_flutter` (ADR-0019 section 8).
/// It owns a `MessageProcessor`, reports Button actions on [onAction] and
/// anything the renderer rejects on [onError]; the host flips the frame to its
/// fallback text on the first error. It sends nothing itself.
class A2uiSurfaceView extends StatefulWidget {
  const A2uiSurfaceView({
    super.key,
    required this.messages,
    required this.catalog,
    this.onAction,
    this.onError,
  });

  /// The surface's protocol messages, as decoded JSON maps.
  final List<Map<String, Object?>> messages;

  /// The assistant catalog (`assistantCatalog()`).
  final CatalogImplementation catalog;

  final ValueChanged<SurfaceAction>? onAction;

  /// A processing error (`A2uiError`) or a runtime one (`A2uiRendererError`).
  final ValueChanged<Object>? onError;

  @override
  State<A2uiSurfaceView> createState() => _A2uiSurfaceViewState();
}

class _A2uiSurfaceViewState extends State<A2uiSurfaceView> {
  MessageProcessor<ComponentImplementation>? _processor;
  SurfaceModel<ComponentImplementation>? _surface;
  final List<Subscription> _subscriptions = [];

  @override
  void initState() {
    super.initState();
    _start();
  }

  @override
  void didUpdateWidget(A2uiSurfaceView oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (!identical(oldWidget.catalog, widget.catalog) ||
        jsonEncode(oldWidget.messages) != jsonEncode(widget.messages)) {
      _stop();
      _start();
    }
  }

  @override
  void dispose() {
    _stop();
    super.dispose();
  }

  void _start() {
    final processor = MessageProcessor<ComponentImplementation>(
      catalogs: [widget.catalog],
    );
    _processor = processor;
    _subscriptions
      ..add(processor.model.onSurfaceCreated.subscribe((s) => _surface = s))
      ..add(processor.model.onAction.subscribe(_handleAction))
      ..add(processor.model.onError.subscribe(_report));
    try {
      processor.processMessages(widget.messages);
    } on A2uiError catch (error) {
      // Not during build: the host may call setState in the callback.
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (mounted) _report(error);
      });
    }
  }

  void _stop() {
    for (final subscription in _subscriptions) {
      subscription.unsubscribe();
    }
    _subscriptions.clear();
    _processor?.dispose();
    _processor = null;
    _surface = null;
  }

  void _report(Object error) => widget.onError?.call(error);

  void _handleAction(A2uiRendererAction action) {
    final surface = _surface;
    final sendModel = surface != null && surface.sendDataModel;
    final model = sendModel ? surface.dataModel.get('/') : null;
    widget.onAction?.call(
      SurfaceAction(
        action,
        dataModel: model is Map ? Map<String, Object?>.from(model) : null,
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final surface = _surface;
    if (surface == null) return const SizedBox.shrink();
    return AssistantSurfaceTheme(child: A2uiSurface(surface: surface));
  }
}
