// RED→GREEN: openspec/changes/generative-ui-a2ui PR 14.5.
import 'package:assistant_app/features/chat/a2ui/a2ui_surface_view.dart';
import 'package:assistant_app/features/chat/a2ui/assistant_catalog.dart';
import 'package:assistant_app/shared/theme/assistant_spacing.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';
// ignore: deprecated_member_use
import 'package:flutter/material.dart' as framework;

import 'a2ui_test_support.dart';

List<Map<String, Object?>> _form() => surfaceMessages(
  [
    {
      'id': 'root',
      'component': 'Column',
      'children': ['field', 'go'],
    },
    {
      'id': 'field',
      'component': 'TextField',
      'label': 'Name',
      'value': {'@path': '/name'},
    },
    {
      'id': 'go',
      'component': 'Button',
      'child': 'label',
      'variant': 'primary',
      'action': {
        'event': {'name': 'go'},
      },
    },
    {'id': 'label', 'component': 'Text', 'text': 'Go'},
  ],
  dataModel: {'name': ''},
);

Widget _view() => A2uiSurfaceView(
  messages: _form(),
  catalog: assistantCatalog(imageLoader: FakeAttachmentImageLoader()),
);

void main() {
  // Material surface widgets on iOS and macOS are the accepted v1 deviation
  // (ADR-0019 section 8). Pinned so a later switch to Cupertino is deliberate.
  for (final platform in [TargetPlatform.iOS, TargetPlatform.macOS]) {
    testWidgets('surface widgets stay Material on $platform', (tester) async {
      debugDefaultTargetPlatformOverride = platform;
      addTearDown(() => debugDefaultTargetPlatformOverride = null);

      await tester.pumpWidget(host(_view()));
      await tester.pump();

      expect(find.byType(framework.TextField), findsOneWidget);
      expect(find.byType(framework.FilledButton), findsOneWidget);
    });
  }

  testWidgets('shapes and padding come from AssistantRadius and Spacing', (
    tester,
  ) async {
    await tester.pumpWidget(host(_view()));
    await tester.pump();

    final theme = framework.Theme.of(
      tester.element(find.byType(framework.FilledButton)),
    );
    final shape =
        theme.filledButtonTheme.style!.shape!.resolve({})!
            as framework.RoundedRectangleBorder;

    expect(
      shape.borderRadius,
      framework.BorderRadius.circular(AssistantRadius.sm),
    );
    expect(
      theme.filledButtonTheme.style!.padding!.resolve({}),
      const framework.EdgeInsets.symmetric(
        horizontal: AssistantSpacing.lg,
        vertical: AssistantSpacing.sm,
      ),
    );
    expect(
      theme.cardTheme.shape,
      framework.RoundedRectangleBorder(
        borderRadius: framework.BorderRadius.circular(AssistantRadius.md),
      ),
    );
  });

  testWidgets('colours follow the active brightness', (tester) async {
    await tester.pumpWidget(host(_view(), brightness: Brightness.dark));
    await tester.pump();

    final theme = framework.Theme.of(
      tester.element(find.byType(framework.FilledButton)),
    );
    expect(theme.brightness, Brightness.dark);
  });
}
