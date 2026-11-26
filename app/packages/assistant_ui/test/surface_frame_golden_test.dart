@Tags(['golden'])
library;

import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import 'support/golden.dart';

/// Placeholder for the rendered A2UI tree: a field and a button.
const _form = Column(
  mainAxisSize: MainAxisSize.min,
  crossAxisAlignment: CrossAxisAlignment.start,
  children: [
    TextField(decoration: InputDecoration(labelText: 'Destination')),
    SizedBox(height: 12),
    FilledButton(onPressed: null, child: Text('Send')),
  ],
);

SurfaceFrame _frame({
  SurfaceStatus status = SurfaceStatus.active,
  bool answerable = true,
  bool hasError = false,
  bool pending = false,
  bool awaitingInput = false,
  Widget? content = _form,
}) => SurfaceFrame(
  title: 'Trip details',
  status: status,
  answerable: answerable,
  fallbackText: 'Where would you like to go, and when?',
  hasError: hasError,
  pending: pending,
  awaitingInput: awaitingInput,
  content: content,
);

void main() {
  final cases = <String, SurfaceFrame>{
    'surface_frame_active': _frame(),
    'surface_frame_awaiting_input': _frame(awaitingInput: true),
    'surface_frame_pending': _frame(pending: true),
    'surface_frame_submitted': _frame(
      status: SurfaceStatus.submitted,
      answerable: false,
    ),
    'surface_frame_closed': _frame(answerable: false),
    'surface_frame_error': _frame(hasError: true),
    'surface_frame_fallback_only': _frame(content: null),
  };

  for (final entry in cases.entries) {
    for (final brightness in Brightness.values) {
      testWidgets('${entry.key} (${brightness.name})', (tester) async {
        await pumpGolden(
          tester,
          Padding(padding: const EdgeInsets.all(16), child: entry.value),
          size: const Size(400, 260),
          brightness: brightness,
        );
        await tester.pump(const Duration(milliseconds: 50));

        await expectLater(
          find.byType(MaterialApp),
          matchesGoldenFile('goldens/${entry.key}_${brightness.name}.png'),
        );
      });
    }
  }
}
