// Tests for [toolPermissionRulesProvider]: the caller's saved tool
// permission rules in the current space, and optimistic revoke.

import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';

import 'package:assistant_api/assistant_api.dart'
    show ToolPermissionRuleSummary;
import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/api/api_client.dart';
import 'package:assistant_app/features/connection/connection_provider.dart';
import 'package:assistant_app/features/integrations/caller_roles.dart';
import 'package:assistant_app/features/settings/tool_permission_rules_provider.dart';
import 'package:assistant_app/features/spaces/space_provider.dart';

class _Selection extends SpaceSelectionNotifier {
  @override
  SpaceSelection build() =>
      const SpaceSelection(orgId: 'org-1', spaceId: 's-1');
}

/// Scripted adapter: answers `GET` with [rules] (or [listError]) and `DELETE`
/// with [deleteStatus], optionally waiting for [deleteGate] first.
class _Adapter implements HttpClientAdapter {
  _Adapter(this.rules);

  List<Map<String, Object?>> rules;
  int? listStatus;
  int deleteStatus = 204;
  Completer<void>? deleteGate;
  final List<RequestOptions> requests = [];

  @override
  Future<ResponseBody> fetch(
    RequestOptions options,
    Stream<Uint8List>? requestStream,
    Future<dynamic>? cancelFuture,
  ) async {
    requests.add(options);
    if (options.method == 'DELETE') {
      await deleteGate?.future;
      return ResponseBody.fromString('', deleteStatus);
    }
    return ResponseBody.fromString(
      listStatus == null ? jsonEncode(rules) : '{"error":"boom"}',
      listStatus ?? 200,
      headers: {
        'content-type': ['application/json'],
      },
    );
  }

  @override
  void close({bool force = false}) {}
}

Map<String, Object?> _rule(String id, String effect, String rule) => {
  'id': id,
  'user_id': 'u-1',
  'effect': effect,
  'rule': rule,
  'tool_name': rule.split('(').first,
  'created_at': '2026-01-02T03:04:05Z',
};

(ProviderContainer, _Adapter) _container(List<Map<String, Object?>> rules) {
  final adapter = _Adapter(rules);
  final client = ApiClient(baseUrl: 'http://fake', token: 'tok');
  client.dioForTesting.httpClientAdapter = adapter;
  final container = ProviderContainer(
    overrides: [apiClientProvider.overrideWithValue(client)],
  );
  addTearDown(container.dispose);
  return (container, adapter);
}

(ProviderContainer, _Adapter) _spaceContainer(
  List<Map<String, Object?>> rules, {
  String role = 'space_admin',
}) {
  final adapter = _Adapter(rules);
  final client = ApiClient(baseUrl: 'http://fake', token: 'tok');
  client.dioForTesting.httpClientAdapter = adapter;
  final container = ProviderContainer(
    overrides: [
      apiClientProvider.overrideWithValue(client),
      spaceSelectionProvider.overrideWith(_Selection.new),
      callerRolesProvider.overrideWith(
        (ref) async => CallerRoles(spaces: const [], roles: {'s-1': role}),
      ),
    ],
  );
  addTearDown(container.dispose);
  return (container, adapter);
}

List<String> _ids(AsyncValue<List<ToolPermissionRuleSummary>> v) =>
    v.requireValue.map((r) => r.id).toList();

void main() {
  test('loads the caller\'s rules', () async {
    final (container, adapter) = _container([
      _rule('r-1', 'allow', 'bash(git status*)'),
      _rule('r-2', 'deny', 'bash(rm *)'),
    ]);

    final rules = await container.read(toolPermissionRulesProvider.future);

    expect(rules.map((r) => r.rule), ['bash(git status*)', 'bash(rm *)']);
    expect(adapter.requests.single.path, '/api/tool-permission-rules');
    expect(adapter.requests.single.queryParameters, isEmpty);
  });

  test('is empty when the caller has no rules', () async {
    final (container, _) = _container([]);

    expect(await container.read(toolPermissionRulesProvider.future), isEmpty);
  });

  test('exposes a load failure as an error', () async {
    final (container, adapter) = _container([]);
    adapter.listStatus = 500;
    container.listen(toolPermissionRulesProvider, (_, _) {});

    await expectLater(
      container.read(toolPermissionRulesProvider.future),
      throwsA(isA<DioException>()),
    );
    expect(container.read(toolPermissionRulesProvider).hasError, isTrue);
  });

  test('reloads when the selected space changes', () async {
    final (container, adapter) = _container([]);
    container.listen(toolPermissionRulesProvider, (_, _) {});
    await container.read(toolPermissionRulesProvider.future);

    container
        .read(spaceSelectionProvider.notifier)
        .selectSpace(spaceId: 's-2', spaceName: 'Other');
    await container.read(toolPermissionRulesProvider.future);

    expect(adapter.requests.where((r) => r.method == 'GET'), hasLength(2));
  });

  group('revoke', () {
    test('removes the row at once and calls DELETE', () async {
      final (container, adapter) = _container([
        _rule('r-1', 'allow', 'bash(git status*)'),
        _rule('r-2', 'deny', 'bash(rm *)'),
      ]);
      container.listen(toolPermissionRulesProvider, (_, _) {});
      await container.read(toolPermissionRulesProvider.future);
      adapter.deleteGate = Completer<void>();

      final pending = container
          .read(toolPermissionRulesProvider.notifier)
          .revoke('r-1');
      await Future<void>.delayed(Duration.zero);

      expect(_ids(container.read(toolPermissionRulesProvider)), ['r-2']);

      adapter.deleteGate!.complete();
      await pending;

      expect(_ids(container.read(toolPermissionRulesProvider)), ['r-2']);
      final delete = adapter.requests.last;
      expect(delete.method, 'DELETE');
      expect(delete.path, '/api/tool-permission-rules/r-1');
    });

    test('puts the row back and rethrows when DELETE fails', () async {
      final (container, adapter) = _container([
        _rule('r-1', 'allow', 'bash(git status*)'),
        _rule('r-2', 'deny', 'bash(rm *)'),
      ]);
      container.listen(toolPermissionRulesProvider, (_, _) {});
      await container.read(toolPermissionRulesProvider.future);
      adapter.deleteStatus = 500;

      await expectLater(
        container.read(toolPermissionRulesProvider.notifier).revoke('r-1'),
        throwsA(isA<DioException>()),
      );

      expect(_ids(container.read(toolPermissionRulesProvider)), ['r-1', 'r-2']);
    });
  });

  group('spaceToolPermissionRulesProvider', () {
    test('starts in loading', () async {
      final (container, _) = _spaceContainer([
        _rule('r-1', 'allow', 'bash(git status*)'),
      ]);
      container.listen(spaceToolPermissionRulesProvider, (_, _) {});

      expect(
        container.read(spaceToolPermissionRulesProvider).isLoading,
        isTrue,
      );

      await container.read(spaceToolPermissionRulesProvider.future);
    });

    test('loads every member\'s rules with scope=space', () async {
      final (container, adapter) = _spaceContainer([
        _rule('r-1', 'allow', 'bash(git status*)'),
        _rule('r-2', 'deny', 'bash(rm *)'),
      ]);

      final rules = await container.read(
        spaceToolPermissionRulesProvider.future,
      );

      expect(rules.map((r) => r.id), ['r-1', 'r-2']);
      expect(adapter.requests.single.path, '/api/tool-permission-rules');
      expect(adapter.requests.single.queryParameters['scope'], 'space');
    });

    test('is empty when the space has no rules', () async {
      final (container, _) = _spaceContainer([]);

      expect(
        await container.read(spaceToolPermissionRulesProvider.future),
        isEmpty,
      );
    });

    test('exposes a load failure as an error', () async {
      final (container, adapter) = _spaceContainer([]);
      adapter.listStatus = 500;
      container.listen(spaceToolPermissionRulesProvider, (_, _) {});

      await expectLater(
        container.read(spaceToolPermissionRulesProvider.future),
        throwsA(isA<DioException>()),
      );
      expect(container.read(spaceToolPermissionRulesProvider).hasError, isTrue);
    });

    test('is never requested for a member or viewer', () async {
      for (final role in ['member', 'viewer']) {
        final (container, adapter) = _spaceContainer([], role: role);

        final rules = await container.read(
          spaceToolPermissionRulesProvider.future,
        );

        expect(rules, isEmpty, reason: 'role $role');
        expect(adapter.requests, isEmpty, reason: 'role $role');
      }
    });

    test('is requested for an org admin', () async {
      final (container, adapter) = _spaceContainer([
        _rule('r-1', 'allow', 'bash(git status*)'),
      ], role: 'org_admin');

      await container.read(spaceToolPermissionRulesProvider.future);

      expect(adapter.requests.single.queryParameters['scope'], 'space');
    });

    group('revoke', () {
      test('removes the row at once and calls DELETE', () async {
        final (container, adapter) = _spaceContainer([
          _rule('r-1', 'allow', 'bash(git status*)'),
          _rule('r-2', 'deny', 'bash(rm *)'),
        ]);
        container.listen(spaceToolPermissionRulesProvider, (_, _) {});
        await container.read(spaceToolPermissionRulesProvider.future);
        adapter.deleteGate = Completer<void>();

        final pending = container
            .read(spaceToolPermissionRulesProvider.notifier)
            .revoke('r-1');
        await Future<void>.delayed(Duration.zero);

        expect(_ids(container.read(spaceToolPermissionRulesProvider)), ['r-2']);

        adapter.deleteGate!.complete();
        await pending;

        final delete = adapter.requests.last;
        expect(delete.method, 'DELETE');
        expect(delete.path, '/api/tool-permission-rules/r-1');
      });

      test('puts the row back and rethrows when DELETE fails', () async {
        final (container, adapter) = _spaceContainer([
          _rule('r-1', 'allow', 'bash(git status*)'),
          _rule('r-2', 'deny', 'bash(rm *)'),
        ]);
        container.listen(spaceToolPermissionRulesProvider, (_, _) {});
        await container.read(spaceToolPermissionRulesProvider.future);
        adapter.deleteStatus = 500;

        await expectLater(
          container
              .read(spaceToolPermissionRulesProvider.notifier)
              .revoke('r-1'),
          throwsA(isA<DioException>()),
        );

        expect(_ids(container.read(spaceToolPermissionRulesProvider)), [
          'r-1',
          'r-2',
        ]);
      });
    });
  });
}
