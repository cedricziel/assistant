import 'package:assistant_api/assistant_api.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/features/admin/admin_providers.dart';
import 'package:assistant_app/features/admin/admin_screen.dart';

// ---------------------------------------------------------------------------
// Stubs

UserSummary _makeUser({
  required String id,
  required String name,
  String email = 'test@example.com',
}) {
  return UserSummary(
    (b) => b
      ..id = id
      ..name = name
      ..email = email,
  );
}

MemberEntry _makeMember({
  required String userId,
  String role = 'member',
  String spaceId = 'space-1',
}) {
  return MemberEntry(
    (b) => b
      ..userId = userId
      ..spaceId = spaceId
      ..role = role
      ..createdAt = '2025-01-15',
  );
}

CatalogItemResponse _makeCatalogItem({
  required String id,
  required String name,
  String resourceType = 'skill',
}) {
  return CatalogItemResponse(
    (b) => b
      ..id = id
      ..name = name
      ..description = 'A test item'
      ..resourceType = resourceType
      ..createdAt = '2025-01-15',
  );
}

class _StubUsersNotifier extends AsyncNotifier<List<UserSummary>>
    implements AdminUsersNotifier {
  _StubUsersNotifier(this._users);
  final List<UserSummary> _users;

  @override
  Future<List<UserSummary>> build() async => _users;

  @override
  Future<UserDetail?> inviteUser({
    required String email,
    required String name,
    String? password,
  }) async => null;

  @override
  Future<void> removeUser(String userId) async {}
}

class _StubMembersNotifier extends AsyncNotifier<List<MemberEntry>>
    implements AdminMembersNotifier {
  _StubMembersNotifier(this._members);
  final List<MemberEntry> _members;

  @override
  Future<List<MemberEntry>> build() async => _members;

  @override
  Future<MemberEntry?> addMember({
    required String userId,
    required String role,
  }) async => null;

  @override
  Future<void> removeMember(String userId) async {}
}

class _StubCatalogNotifier extends AsyncNotifier<List<CatalogItemResponse>>
    implements AdminCatalogNotifier {
  _StubCatalogNotifier(this._items);
  final List<CatalogItemResponse> _items;

  @override
  Future<List<CatalogItemResponse>> build() async => _items;

  @override
  Future<CatalogItemResponse?> publishItem({
    required String name,
    required String resourceType,
    String? description,
  }) async => null;

  @override
  Future<void> removeItem(String itemId) async {}
}

class _StubSpacesNotifier extends AsyncNotifier<List<SpaceSummary>>
    implements AdminSpacesNotifier {
  _StubSpacesNotifier(this._spaces);
  final List<SpaceSummary> _spaces;

  @override
  Future<List<SpaceSummary>> build() async => _spaces;

  @override
  Future<SpaceDetail?> createSpace({
    required String name,
    required String slug,
  }) async => null;

  @override
  Future<void> deleteSpace(String spaceId) async {}
}

Widget _buildApp({
  List<UserSummary> users = const [],
  List<MemberEntry> members = const [],
  List<CatalogItemResponse> catalogItems = const [],
  List<SpaceSummary> spaces = const [],
}) {
  return ProviderScope(
    overrides: [
      adminUsersProvider.overrideWith(() => _StubUsersNotifier(users)),
      adminMembersProvider.overrideWith(() => _StubMembersNotifier(members)),
      adminCatalogProvider.overrideWith(
        () => _StubCatalogNotifier(catalogItems),
      ),
      adminSpacesProvider.overrideWith(() => _StubSpacesNotifier(spaces)),
    ],
    child: const MaterialApp(home: AdminScreen()),
  );
}

// ---------------------------------------------------------------------------
// Tests

void main() {
  group('AdminScreen', () {
    testWidgets('shows tab bar with three tabs', (tester) async {
      await tester.pumpWidget(_buildApp());
      await tester.pumpAndSettle();

      expect(find.text('Users'), findsOneWidget);
      expect(find.text('Spaces'), findsOneWidget);
      expect(find.text('Catalog'), findsOneWidget);
    });

    testWidgets('app bar shows Admin title', (tester) async {
      await tester.pumpWidget(_buildApp());
      await tester.pumpAndSettle();

      expect(find.text('Admin'), findsOneWidget);
    });

    testWidgets('Users tab shows user list', (tester) async {
      await tester.pumpWidget(
        _buildApp(
          users: [
            _makeUser(id: '1', name: 'Alice', email: 'alice@example.com'),
            _makeUser(id: '2', name: 'Bob', email: 'bob@example.com'),
          ],
        ),
      );
      await tester.pumpAndSettle();

      expect(find.text('Alice'), findsOneWidget);
      expect(find.text('Bob'), findsOneWidget);
    });

    testWidgets('Users tab shows empty state', (tester) async {
      await tester.pumpWidget(_buildApp());
      await tester.pumpAndSettle();

      expect(find.text('No users'), findsOneWidget);
    });

    testWidgets('Users tab has invite button', (tester) async {
      await tester.pumpWidget(_buildApp());
      await tester.pumpAndSettle();

      expect(find.byType(FloatingActionButton), findsOneWidget);
    });

    testWidgets('Spaces tab shows space list', (tester) async {
      final spaces = [
        SpaceSummary(
          (b) => b
            ..id = 's1'
            ..name = 'Production'
            ..slug = 'prod',
        ),
      ];
      await tester.pumpWidget(_buildApp(spaces: spaces));
      await tester.pumpAndSettle();

      // Navigate to Spaces tab.
      await tester.tap(find.text('Spaces'));
      await tester.pumpAndSettle();

      expect(find.text('Production'), findsOneWidget);
    });

    testWidgets('Catalog tab shows catalog items', (tester) async {
      await tester.pumpWidget(
        _buildApp(
          catalogItems: [
            _makeCatalogItem(id: 'c1', name: 'My Skill', resourceType: 'skill'),
          ],
        ),
      );
      await tester.pumpAndSettle();

      // Navigate to Catalog tab.
      await tester.tap(find.text('Catalog'));
      await tester.pumpAndSettle();

      expect(find.text('My Skill'), findsOneWidget);
    });

    testWidgets('FAB opens invite dialog on Users tab', (tester) async {
      await tester.pumpWidget(_buildApp());
      await tester.pumpAndSettle();

      await tester.tap(find.byType(FloatingActionButton));
      await tester.pumpAndSettle();

      expect(find.text('Invite user'), findsOneWidget);
      expect(find.text('Email'), findsOneWidget);
      expect(find.text('Name'), findsOneWidget);
    });

    testWidgets('Members tab shows member list', (tester) async {
      await tester.pumpWidget(
        _buildApp(
          members: [
            _makeMember(userId: 'u1', role: 'member'),
            _makeMember(userId: 'u2', role: 'space-admin'),
          ],
        ),
      );
      await tester.pumpAndSettle();

      // Navigate to Spaces tab, then select Members sub-section.
      await tester.tap(find.text('Spaces'));
      await tester.pumpAndSettle();

      // Members section is shown below spaces on the Spaces tab.
      expect(find.text('Members'), findsWidgets);
    });
  });
}
