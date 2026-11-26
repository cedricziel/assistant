@Tags(['golden'])
library;

import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import 'support/golden.dart';

const _size = Size(420, 60);

MessageActionButton _button(
  String label, {
  IconData icon = Icons.copy_outlined,
  MessageActionTone tone = MessageActionTone.muted,
  bool selected = false,
}) => MessageActionButton(
  icon: icon,
  label: label,
  tone: tone,
  selected: selected,
  onPressed: () {},
);

Future<void> _golden(
  WidgetTester tester,
  String name,
  Widget widget, {
  Brightness brightness = Brightness.light,
}) async {
  await pumpGolden(
    tester,
    Padding(padding: const EdgeInsets.all(12), child: widget),
    size: _size,
    brightness: brightness,
  );
  await tester.pump(const Duration(milliseconds: 50));
  await expectLater(
    find.byType(MaterialApp),
    matchesGoldenFile('goldens/$name.png'),
  );
}

void main() {
  testWidgets('MessageActionButton muted', (tester) async {
    await _golden(tester, 'message_action_button_muted', _button('Copy'));
  });

  testWidgets('MessageActionButton danger', (tester) async {
    await _golden(
      tester,
      'message_action_button_danger',
      _button('Retry', icon: Icons.refresh, tone: MessageActionTone.danger),
    );
  });

  testWidgets('MessageActionButton selected', (tester) async {
    await _golden(
      tester,
      'message_action_button_selected',
      _button('Good', icon: Icons.thumb_up, selected: true),
    );
  });

  testWidgets('MessageActionButton selected, dark', (tester) async {
    await _golden(
      tester,
      'message_action_button_selected_dark',
      _button('Good', icon: Icons.thumb_up, selected: true),
      brightness: Brightness.dark,
    );
  });

  testWidgets('MessageActions full row', (tester) async {
    await _golden(
      tester,
      'message_actions_full',
      MessageActions(
        leading: const Text('1 / 2', style: TextStyle(fontSize: 12)),
        actions: [
          _button('Copy'),
          _button('Share', icon: Icons.share_outlined),
          _button('Retry', icon: Icons.refresh, tone: MessageActionTone.danger),
        ],
        trailing: [
          const Text('1.2 s', style: TextStyle(fontSize: 12)),
          _button('Good', icon: Icons.thumb_up, selected: true),
        ],
      ),
    );
  });
}
