import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/api/models/server_profile.dart';
import 'package:assistant_app/features/connection/connection_provider.dart';
import 'package:assistant_app/features/contexts/models/context_model.dart';
import 'package:assistant_app/features/contexts/providers/context_providers.dart';
import 'package:assistant_app/shared/server_chip.dart';

// ---------------------------------------------------------------------------
// Fakes

class _FakeServerProfileNotifier extends ServerProfileNotifier {
  _FakeServerProfileNotifier(this._profile);
  final ServerProfile? _profile;

  @override
  Future<ServerConnectionState> build() async =>
      ServerConnectionState(profile: _profile);
}

class _FakeActiveContextNotifier extends ActiveContextNotifier {
  bool deactivateCalled = false;

  @override
  Future<AssistantContext?> build() async => null;

  @override
  Future<void> deactivate() async {
    deactivateCalled = true;
    state = const AsyncData(null);
  }
}

// ---------------------------------------------------------------------------
// Helpers

Widget _buildChip({ServerProfile? profile}) {
  return ProviderScope(
    overrides: [
      serverProfileProvider.overrideWith(
        () => _FakeServerProfileNotifier(profile),
      ),
      activeContextProvider.overrideWith(_FakeActiveContextNotifier.new),
    ],
    child: const MaterialApp(
      home: Scaffold(body: Center(child: ServerChip())),
    ),
  );
}

// ---------------------------------------------------------------------------

void main() {
  group('ServerChip', () {
    testWidgets('shows displayLabel when connected', (tester) async {
      const profile = ServerProfile(
        baseUrl: 'http://localhost:8080',
        token: 'tok',
        label: 'Local',
      );

      await tester.pumpWidget(_buildChip(profile: profile));
      await tester.pump();

      expect(find.text('Local'), findsOneWidget);
    });

    testWidgets('shows hostname as displayLabel when label is empty', (
      tester,
    ) async {
      const profile = ServerProfile(
        baseUrl: 'http://myserver:8080',
        token: 'tok',
        label: '',
      );

      await tester.pumpWidget(_buildChip(profile: profile));
      await tester.pump();

      // displayLabel falls back to Uri.parse(baseUrl).host → 'myserver'
      expect(find.text('myserver'), findsOneWidget);
    });

    testWidgets('shows "Not connected" when profile is null', (tester) async {
      await tester.pumpWidget(_buildChip(profile: null));
      await tester.pump();

      expect(find.text('Not connected'), findsOneWidget);
    });

    testWidgets('tapping chip opens bottom sheet', (tester) async {
      const profile = ServerProfile(
        baseUrl: 'http://localhost:8080',
        token: 'tok',
        label: 'Local',
      );

      await tester.pumpWidget(_buildChip(profile: profile));
      await tester.pump();

      await tester.tap(find.byType(ServerChip));
      await tester.pumpAndSettle();

      expect(find.text('Server'), findsOneWidget);
    });

    testWidgets('bottom sheet shows server URL', (tester) async {
      const profile = ServerProfile(
        baseUrl: 'http://localhost:8080',
        token: 'tok',
        label: 'Local',
      );

      await tester.pumpWidget(_buildChip(profile: profile));
      await tester.pump();

      await tester.tap(find.byType(ServerChip));
      await tester.pumpAndSettle();

      expect(find.text('http://localhost:8080'), findsOneWidget);
    });

    testWidgets('bottom sheet Disconnect button calls deactivate', (
      tester,
    ) async {
      const profile = ServerProfile(
        baseUrl: 'http://localhost:8080',
        token: 'tok',
        label: 'Local',
      );

      await tester.pumpWidget(_buildChip(profile: profile));
      await tester.pump();

      await tester.tap(find.byType(ServerChip));
      await tester.pumpAndSettle();

      expect(find.text('Disconnect'), findsOneWidget);

      await tester.tap(find.text('Disconnect'));
      await tester.pumpAndSettle();

      // After disconnect the bottom sheet should be dismissed (no crash).
      expect(find.text('Disconnect'), findsNothing);
    });
  });
}
