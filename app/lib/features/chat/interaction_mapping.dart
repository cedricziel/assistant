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

/// Whether a `question` response body says "I won't answer".
bool declinedFrom(Map<String, Object?>? response) =>
    response?['declined'] == true;

/// The owner's answers in a `question` response body.
List<QuestionAnswer> questionAnswersFrom(Map<String, Object?>? response) {
  final raw = response?['answers'];
  if (raw is! List) return const [];
  return [
    for (final item in raw)
      if (item is Map)
        QuestionAnswer(
          questionId: item['question_id'] is String
              ? item['question_id']! as String
              : '',
          selectedOptionIds: [
            if (item['selected_option_ids'] is List)
              for (final id in item['selected_option_ids']! as List)
                if (id is String) id,
          ],
          text: item['text'] is String ? item['text'] as String : null,
        ),
  ];
}

/// The questions of a `question` request body, in request order.
List<QuestionItem> _questionsFrom(Object? raw) {
  if (raw is! List) return const [];
  return [
    for (final item in raw)
      if (item is Map)
        QuestionItem(
          id: item['id'] is String ? item['id']! as String : '',
          prompt: item['prompt'] is String ? item['prompt']! as String : '',
          options: _optionsFrom(item['options']),
          multiple: item['multiple'] == true,
          freeText: item['free_text'] == true,
        ),
  ];
}

List<QuestionOption> _optionsFrom(Object? raw) {
  if (raw is! List) return const [];
  return [
    for (final item in raw)
      if (item is Map)
        QuestionOption(
          id: item['id'] is String ? item['id']! as String : '',
          label: item['label'] is String ? item['label']! as String : '',
          description: item['description'] is String
              ? item['description']! as String
              : null,
        ),
  ];
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
  // Kind-specific fields exist only on their own kind; a newer server may put
  // any JSON type in a kind this build does not know, so cast nothing else.
  final isApproval = kind == InteractionKind.toolApproval.wire;
  final isQuestion = kind == InteractionKind.question.wire;
  final options = isApproval ? request['options'] : null;
  final resolved = status == InteractionStatus.resolved;
  return InteractionRecord(
    id: id,
    kind: InteractionKind.fromWire(kind),
    status: status,
    createdAt: createdAt,
    expiresAt: expiresAt,
    toolCallId: toolCallId,
    rendersCard: rendersCard,
    toolName: isApproval ? request['tool_name'] as String? : null,
    arguments: arguments is Map ? arguments.cast<String, Object?>() : const {},
    argumentsTruncated: isApproval && request['arguments_truncated'] == true,
    reason: isApproval ? request['reason'] as String? : null,
    options: [
      if (options is List)
        for (final o in options)
          if (o is String) ?ApprovalDecision.fromWire(o),
    ],
    suggestedRule: isApproval ? request['suggested_rule'] as String? : null,
    decision: resolved ? approvalDecisionFrom(response) : null,
    resolvedAt: resolvedAt,
    ruleId: ruleId,
    questions: isQuestion ? _questionsFrom(request['questions']) : const [],
    answers: isQuestion && resolved ? questionAnswersFrom(response) : const [],
    declined: isQuestion && resolved && declinedFrom(response),
  );
}
