import 'package:assistant_api/assistant_api.dart'
    show InteractionDetail, InteractionStatusView;
import 'package:assistant_ui/models.dart';

import '../../api/models/stream_event.dart';

// Maps the wire shapes of a turn interaction (the `interaction_requested`
// event, the stored `InteractionDetail`) onto the pure [InteractionRecord] the
// elements render. Request fields of kinds this build does not render are left
// out: the record keeps its common fields so the registry can still decide.

/// The record for a freshly requested interaction: always pending.
InteractionRecord interactionRecordFromEvent(InteractionRequestedEvent event) =>
    _record(
      id: event.interactionId,
      kind: event.kind,
      status: InteractionStatus.pending,
      createdAt: event.createdAt,
      expiresAt: event.expiresAt,
      toolCallId: event.toolCallId,
      rendersCard: event.rendersCard,
      request: event.request,
    );

/// The record for a stored interaction, in whatever state it is in.
InteractionRecord interactionRecordFromDetail(InteractionDetail detail) {
  final request = detail.request;
  final response = detail.response;
  return _record(
    id: detail.interactionId,
    kind: detail.kind,
    status: switch (detail.status) {
      InteractionStatusView.pending => InteractionStatus.pending,
      InteractionStatusView.resolved => InteractionStatus.resolved,
      InteractionStatusView.cancelled => InteractionStatus.cancelled,
      InteractionStatusView.expired => InteractionStatus.expired,
      _ => InteractionStatus.pending,
    },
    createdAt: detail.createdAt,
    expiresAt: detail.expiresAt,
    toolCallId: detail.toolCallId,
    rendersCard: detail.rendersCard,
    request: request.isMap ? request.asMap.cast<String, Object?>() : const {},
    response: response != null && response.isMap
        ? response.asMap.cast<String, Object?>()
        : null,
    resolvedAt: detail.resolvedAt,
    ruleId: detail.ruleId,
  );
}

/// The decision an approval `response` body carries, or null.
ApprovalDecision? approvalDecisionFrom(Map<String, Object?>? response) {
  final decision = response?['decision'];
  return decision is String ? ApprovalDecision.fromWire(decision) : null;
}

InteractionRecord _record({
  required String id,
  required String kind,
  required InteractionStatus status,
  required DateTime createdAt,
  required DateTime expiresAt,
  required String? toolCallId,
  required bool rendersCard,
  required Map<String, Object?> request,
  Map<String, Object?>? response,
  DateTime? resolvedAt,
  String? ruleId,
}) {
  final arguments = request['arguments'];
  final options = request['options'];
  return InteractionRecord(
    id: id,
    kind: InteractionKind.fromWire(kind),
    status: status,
    createdAt: createdAt,
    expiresAt: expiresAt,
    toolCallId: toolCallId,
    rendersCard: rendersCard,
    toolName: request['tool_name'] as String?,
    arguments: arguments is Map ? arguments.cast<String, Object?>() : const {},
    argumentsTruncated: request['arguments_truncated'] == true,
    reason: request['reason'] as String?,
    options: [
      if (options is List)
        for (final o in options)
          if (o is String) ?ApprovalDecision.fromWire(o),
    ],
    suggestedRule: request['suggested_rule'] as String?,
    decision: status == InteractionStatus.resolved
        ? approvalDecisionFrom(response)
        : null,
    resolvedAt: resolvedAt,
    ruleId: ruleId,
  );
}
