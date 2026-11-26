/// What a turn is asking its owner. Mirrors the server's interaction `kind`.
enum InteractionKind {
  toolApproval('tool_approval'),
  question('question'),

  /// A kind this build does not know. Never answered automatically.
  unknown('');

  const InteractionKind(this.wire);

  /// The `kind` string on the wire.
  final String wire;

  /// Maps a wire `kind` to a value; anything unrecognised is [unknown].
  static InteractionKind fromWire(String value) => values.firstWhere(
    (k) => k != unknown && k.wire == value,
    orElse: () => unknown,
  );
}

/// An answer to a tool approval. The enum is closed: the server decides which
/// of these a given request offers.
enum ApprovalDecision {
  allowOnce('allow_once'),
  allowAlways('allow_always'),
  rejectOnce('reject_once'),
  rejectAlways('reject_always');

  const ApprovalDecision(this.wire);

  /// The `decision` string on the wire.
  final String wire;

  /// Whether this decision saves a rule.
  bool get isAlways => this == allowAlways || this == rejectAlways;

  /// Whether this decision lets the tool run.
  bool get isAllow => this == allowOnce || this == allowAlways;

  /// Maps a wire `decision` to a value, or null when unrecognised.
  static ApprovalDecision? fromWire(String value) {
    for (final d in values) {
      if (d.wire == value) return d;
    }
    return null;
  }
}

/// Where an interaction is in its life.
enum InteractionStatus {
  /// Waiting for the owner.
  pending,

  /// The owner answered and the request is in flight.
  submitting,

  /// Answered; [InteractionRecord.decision] says how.
  resolved,

  /// Nobody answered before `expiresAt`.
  expired,

  /// The turn ended or was stopped first.
  cancelled,
}

/// One interaction a turn put to its owner, as the elements render it.
///
/// Pure data: the app maps the wire request, the resolution and the stored
/// detail into this. The approval fields ([toolName] and below) are used when
/// [kind] is [InteractionKind.toolApproval].
class InteractionRecord {
  const InteractionRecord({
    required this.id,
    required this.kind,
    required this.status,
    required this.createdAt,
    required this.expiresAt,
    this.toolCallId,
    this.toolName,
    this.arguments = const {},
    this.argumentsTruncated = false,
    this.reason,
    this.options = const [],
    this.suggestedRule,
    this.decision,
    this.resolvedAt,
    this.ruleId,
    this.rendersCard = true,
  });

  final String id;
  final InteractionKind kind;
  final InteractionStatus status;
  final DateTime createdAt;
  final DateTime expiresAt;

  /// The tool call this approval gates, when the provider supplied an id.
  final String? toolCallId;
  final String? toolName;

  /// The call's arguments; the server may have shortened them.
  final Map<String, Object?> arguments;
  final bool argumentsTruncated;

  /// Why the permission pipeline asked.
  final String? reason;

  /// The answers the server allows for this request.
  final List<ApprovalDecision> options;

  /// The rule an "always" answer would save. Null when no "always" is offered.
  final String? suggestedRule;

  /// How the owner answered, once [status] is [InteractionStatus.resolved].
  final ApprovalDecision? decision;
  final DateTime? resolvedAt;

  /// The rule that the answer saved, if any.
  final String? ruleId;

  /// False when the kind wants no timeline card. The turn still waits.
  final bool rendersCard;
}
