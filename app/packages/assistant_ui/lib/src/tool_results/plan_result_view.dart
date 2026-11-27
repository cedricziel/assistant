import 'package:material_ui/material_ui.dart';

import '../models/chat_message.dart';
import '../theme/assistant_ui_spacing.dart';
import 'tool_result_renderer.dart';

/// Longest plan title the contract allows, in characters.
const _maxPlanTitleChars = 120;

/// Longest step title the contract allows, in characters.
const _maxStepTitleChars = 200;

/// Most steps the contract allows.
const _maxSteps = 50;

/// Status of one plan step, as the `plan` contract names it.
enum PlanStepStatus {
  pending('Pending'),
  inProgress('In progress'),
  completed('Completed'),
  failed('Failed'),
  skipped('Skipped');

  const PlanStepStatus(this.label);

  /// Human-readable status, used in semantics labels.
  final String label;

  static PlanStepStatus? fromWire(Object? value) => switch (value) {
    'pending' => pending,
    'in_progress' => inProgress,
    'completed' => completed,
    'failed' => failed,
    'skipped' => skipped,
    _ => null,
  };
}

/// One step of a `plan` result.
class PlanStep {
  const PlanStep({required this.title, required this.status});

  final String title;
  final PlanStepStatus status;
}

/// The `plan` contract: an ordered list of steps with statuses. See
/// `schemas/tool-results/plan.schema.json`.
class PlanData {
  const PlanData({required this.steps, this.title});

  final String? title;
  final List<PlanStep> steps;

  int get total => steps.length;

  /// Steps whose status is [PlanStepStatus.completed].
  int get completed =>
      steps.where((step) => step.status == PlanStepStatus.completed).length;

  /// `completed/total`, such as `3/5`.
  String get progress => '$completed/$total';
}

/// Resolves `kind: "plan"` payloads to [PlanResultView].
class PlanResultRenderer extends ToolResultRenderer<PlanData> {
  const PlanResultRenderer();

  @override
  String get kind => 'plan';

  @override
  PlanData? parse(Map<String, Object?> data) {
    final title = data['title'];
    final rawSteps = data['steps'];
    if (data['kind'] != kind ||
        (title != null &&
            (title is! String || title.runes.length > _maxPlanTitleChars)) ||
        rawSteps is! List ||
        rawSteps.length > _maxSteps) {
      return null;
    }
    final steps = <PlanStep>[];
    for (final rawStep in rawSteps) {
      if (rawStep is! Map) return null;
      final stepTitle = rawStep['title'];
      final status = PlanStepStatus.fromWire(rawStep['status']);
      if (stepTitle is! String ||
          stepTitle.runes.length > _maxStepTitleChars ||
          status == null) {
        return null;
      }
      steps.add(PlanStep(title: stepTitle, status: status));
    }
    return PlanData(title: title as String?, steps: steps);
  }

  @override
  Widget summary(BuildContext context, PlanData model) {
    final title = model.title;
    final text = title?.trim().isNotEmpty == true
        ? '$title · ${model.progress}'
        : model.progress;
    return Text(
      text,
      maxLines: 1,
      overflow: TextOverflow.ellipsis,
      style: Theme.of(context).textTheme.bodySmall,
    );
  }

  @override
  Widget build(BuildContext context, PlanData model, ToolCallStatus status) =>
      PlanResultView(data: model);
}

/// A read-only working plan: optional title, completed/total progress and one
/// row per step with a status icon and label.
class PlanResultView extends StatelessWidget {
  const PlanResultView({super.key, required this.data});

  final PlanData data;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final title = data.title;
    final hasTitle = title?.trim().isNotEmpty == true;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      mainAxisSize: MainAxisSize.min,
      children: [
        Row(
          children: [
            if (hasTitle)
              Expanded(
                child: Text(
                  title!,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: theme.textTheme.titleSmall,
                ),
              )
            else
              const Spacer(),
            const SizedBox(width: AssistantUiSpacing.sm),
            Text(data.progress, style: theme.textTheme.labelLarge),
          ],
        ),
        const SizedBox(height: AssistantUiSpacing.xs),
        LinearProgressIndicator(
          value: data.total == 0 ? 0 : data.completed / data.total,
          minHeight: 3,
          semanticsLabel: '${data.completed} of ${data.total} steps completed',
        ),
        const SizedBox(height: AssistantUiSpacing.xs),
        for (final step in data.steps) _StepRow(step: step),
      ],
    );
  }
}

class _StepRow extends StatelessWidget {
  const _StepRow({required this.step});

  final PlanStep step;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final color = switch (step.status) {
      PlanStepStatus.completed => theme.colorScheme.primary,
      PlanStepStatus.inProgress => theme.colorScheme.primary,
      PlanStepStatus.failed => theme.colorScheme.error,
      PlanStepStatus.pending ||
      PlanStepStatus.skipped => theme.colorScheme.onSurfaceVariant,
    };
    final icon = switch (step.status) {
      PlanStepStatus.pending => Icons.radio_button_unchecked,
      PlanStepStatus.inProgress => Icons.autorenew,
      PlanStepStatus.completed => Icons.check_circle,
      PlanStepStatus.failed => Icons.cancel,
      PlanStepStatus.skipped => Icons.skip_next,
    };
    return Semantics(
      container: true,
      label: '${step.status.label}: ${step.title}',
      excludeSemantics: true,
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: 2),
        child: Row(
          children: [
            Icon(icon, size: 16, color: color),
            const SizedBox(width: AssistantUiSpacing.xs),
            Expanded(
              child: Text(
                step.title,
                maxLines: 2,
                overflow: TextOverflow.ellipsis,
                style: theme.textTheme.bodySmall?.copyWith(
                  color: switch (step.status) {
                    PlanStepStatus.completed || PlanStepStatus.skipped =>
                      theme.colorScheme.onSurfaceVariant,
                    _ => null,
                  },
                  decoration: switch (step.status) {
                    PlanStepStatus.completed ||
                    PlanStepStatus.skipped => TextDecoration.lineThrough,
                    _ => null,
                  },
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
