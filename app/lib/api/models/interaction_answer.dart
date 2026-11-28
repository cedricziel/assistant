import 'package:assistant_ui/models.dart' show ApprovalDecision, QuestionAnswer;

import 'surface_action.dart';

/// The owner's answer to a turn interaction, as the respond route takes it.
///
/// One subtype per request `kind`. The server validates the answer against the
/// interaction it belongs to, so a decision the request did not offer is
/// refused with a 400.
sealed class InteractionAnswer {
  const InteractionAnswer();
}

/// Answers a `tool_approval` interaction.
final class ToolApprovalAnswer extends InteractionAnswer {
  const ToolApprovalAnswer(this.decision);

  final ApprovalDecision decision;
}

/// Answers a `question` interaction, one entry per question.
final class QuestionAnswers extends InteractionAnswer {
  const QuestionAnswers(this.answers);

  final List<QuestionAnswer> answers;
}

/// Answers a `surface_action` interaction: an action the surface's Button
/// raised, plus the data model when the surface sends one.
final class SurfaceActionAnswer extends InteractionAnswer {
  const SurfaceActionAnswer(this.action);

  final SurfaceActionSubmission action;
}

/// "I won't answer", accepted for any kind. An approval ends as `reject_once`.
final class DeclineAnswer extends InteractionAnswer {
  const DeclineAnswer();
}
