@Tags(['golden'])
library;

import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import 'support/golden.dart';

const _states = <String, ({int index, int count, bool busy})>{
  'first': (index: 0, count: 3, busy: false),
  'middle': (index: 1, count: 3, busy: false),
  'last': (index: 2, count: 3, busy: false),
  'busy': (index: 1, count: 3, busy: true),
};

void main() {
  for (final brightness in Brightness.values) {
    for (final MapEntry(key: name, value: state) in _states.entries) {
      testWidgets('BranchPicker $name ${brightness.name}', (tester) async {
        await pumpGolden(
          tester,
          BranchPicker(
            index: state.index,
            count: state.count,
            busy: state.busy,
            onPrevious: () {},
            onNext: () {},
          ),
          size: const Size(200, 80),
          brightness: brightness,
        );

        await expectLater(
          find.byType(BranchPicker),
          matchesGoldenFile(
            'goldens/branch_picker_${name}_${brightness.name}.png',
          ),
        );
      });
    }
  }
}
