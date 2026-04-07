import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/features/updater/update_banner.dart';
import 'package:assistant_app/features/updater/update_provider.dart';
import 'package:assistant_app/features/updater/update_service.dart';

/// A minimal [ReleaseInfo] for testing.
ReleaseInfo _fakeRelease() => ReleaseInfo(
      tagName: 'v2.0.0',
      body: 'Performance improvements and bug fixes.',
      htmlUrl: 'https://github.com/cedricziel/assistant/releases/tag/v2.0.0',
      assets: const [],
    );

/// A [UpdateInfo] for testing.
UpdateInfo _fakeUpdateInfo() => UpdateInfo(
      currentVersion: '1.0.0',
      latestVersion: 'v2.0.0',
      release: _fakeRelease(),
    );

void main() {
  testWidgets('UpdateBannerWrapper shows banner when update is available',
      (tester) async {
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          updateCheckProvider.overrideWith(() => _FakeUpdateNotifier()),
        ],
        child: const MaterialApp(
          home: Scaffold(
            body: UpdateBannerWrapper(
              child: SizedBox(key: Key('body')),
            ),
          ),
        ),
      ),
    );
    await tester.pump();

    expect(find.text('Update available — v2.0.0'), findsOneWidget);
    expect(find.text('Download'), findsOneWidget);
    expect(find.byKey(const Key('body')), findsOneWidget);
  });

  testWidgets('Dismiss hides the banner', (tester) async {
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          updateCheckProvider.overrideWith(() => _FakeUpdateNotifier()),
        ],
        child: const MaterialApp(
          home: Scaffold(
            body: UpdateBannerWrapper(
              child: SizedBox(key: Key('body')),
            ),
          ),
        ),
      ),
    );
    await tester.pump();

    expect(find.text('Update available — v2.0.0'), findsOneWidget);

    await tester.tap(find.byTooltip('Dismiss'));
    await tester.pump();

    expect(find.text('Update available — v2.0.0'), findsNothing);
  });

  testWidgets('UpdateBannerWrapper shows no banner when up-to-date',
      (tester) async {
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          updateCheckProvider.overrideWith(() => _FakeUpToDateNotifier()),
        ],
        child: const MaterialApp(
          home: Scaffold(
            body: UpdateBannerWrapper(
              child: SizedBox(key: Key('body')),
            ),
          ),
        ),
      ),
    );
    await tester.pump();

    expect(find.text('Update available — v2.0.0'), findsNothing);
    expect(find.byKey(const Key('body')), findsOneWidget);
  });
}

class _FakeUpdateNotifier extends UpdateCheckNotifier {
  @override
  Future<UpdateInfo?> build() async => _fakeUpdateInfo();
}

class _FakeUpToDateNotifier extends UpdateCheckNotifier {
  @override
  Future<UpdateInfo?> build() async => null;
}
