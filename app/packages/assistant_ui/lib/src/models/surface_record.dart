/// Where a surface is in its life. Mirrors the server's stored `status`.
///
/// "Closed" is not a status: it is an `active` surface that is no longer
/// answerable, see [SurfaceRecord.isClosed].
enum SurfaceStatus {
  active('active'),
  submitted('submitted');

  const SurfaceStatus(this.wire);

  /// The `status` string on the wire.
  final String wire;

  /// Maps a wire `status` to a value; anything unrecognised is [active].
  static SurfaceStatus fromWire(String value) => values.firstWhere(
    (s) => s.wire == value,
    orElse: () => SurfaceStatus.active,
  );
}

/// One agent-composed UI surface, as the elements render it.
///
/// Pure data: the app maps the `surface_update` event and the stored
/// `SurfaceSummary` into this. [messages] are A2UI messages (JSON maps) that
/// the app's renderer consumes; this package never interprets them.
class SurfaceRecord {
  const SurfaceRecord({
    required this.surfaceId,
    required this.anchorMessageId,
    required this.anchorCallIndex,
    required this.title,
    required this.fallbackText,
    required this.status,
    required this.answerable,
    this.toolCallId,
    this.messages = const [],
    this.interactionId,
    this.submittedAction,
  });

  final String surfaceId;

  /// The provider's id of the tool call that made the surface, when known.
  final String? toolCallId;

  /// The tool-call message the surface hangs off, and the call's index in it.
  final String anchorMessageId;
  final int anchorCallIndex;

  final String title;

  /// Plain text shown when the surface cannot be rendered.
  final String fallbackText;

  final SurfaceStatus status;

  /// False once a later user message exists. Derived by the server.
  final bool answerable;

  /// A2UI v1.0 messages that build the surface.
  final List<Map<String, Object?>> messages;

  /// The interaction the turn waits on, while it waits.
  final String? interactionId;

  /// The action the user submitted, once [status] is submitted.
  final Map<String, Object?>? submittedAction;

  /// An `active` surface nobody can answer any more. Read-only, labelled
  /// "Closed".
  bool get isClosed => status == SurfaceStatus.active && !answerable;

  SurfaceRecord copyWith({
    String? title,
    String? fallbackText,
    SurfaceStatus? status,
    bool? answerable,
    List<Map<String, Object?>>? messages,
    String? interactionId,
    bool clearInteraction = false,
    Map<String, Object?>? submittedAction,
  }) {
    return SurfaceRecord(
      surfaceId: surfaceId,
      toolCallId: toolCallId,
      anchorMessageId: anchorMessageId,
      anchorCallIndex: anchorCallIndex,
      title: title ?? this.title,
      fallbackText: fallbackText ?? this.fallbackText,
      status: status ?? this.status,
      answerable: answerable ?? this.answerable,
      messages: messages ?? this.messages,
      interactionId: clearInteraction
          ? null
          : (interactionId ?? this.interactionId),
      submittedAction: submittedAction ?? this.submittedAction,
    );
  }
}
