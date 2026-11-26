import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

// tests for BranchInfo
void main() {
  final instance = BranchInfoBuilder();
  // TODO add properties to the builder and call build()

  group(BranchInfo, () {
    // How many versions there are (always at least 2).
    // int count
    test('to test the property `count`', () async {
      // TODO
    });

    // Zero-based position of the active version, in creation order.
    // int index
    test('to test the property `index`', () async {
      // TODO
    });

    // The first message of every version, in creation order. Switching to any of them selects that version.
    // BuiltList<String> siblingIds
    test('to test the property `siblingIds`', () async {
      // TODO
    });
  });
}
