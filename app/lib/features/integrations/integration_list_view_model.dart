import 'package:assistant_api/assistant_api.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../connection/connection_provider.dart';
import 'caller_roles.dart';
import 'integration_failure.dart';

/// Which integrations a list shows.
enum IntegrationScopeKind { org, space, connections }

/// Selects the integrations a list view model loads: the org layer, one space
/// (its own plus the org ones offered to it), or the signed-in user's
/// connections.
class IntegrationListScope {
  const IntegrationListScope.org()
    : kind = IntegrationScopeKind.org,
      spaceId = null;

  const IntegrationListScope.space(String this.spaceId)
    : kind = IntegrationScopeKind.space;

  const IntegrationListScope.connections()
    : kind = IntegrationScopeKind.connections,
      spaceId = null;

  final IntegrationScopeKind kind;
  final String? spaceId;

  @override
  bool operator ==(Object other) =>
      other is IntegrationListScope &&
      other.kind == kind &&
      other.spaceId == spaceId;

  @override
  int get hashCode => Object.hash(kind, spaceId);
}

/// One integration in a list.
class IntegrationRow {
  const IntegrationRow(this.integration, {this.optedOut = false});

  final IntegrationSummary integration;

  /// The space scope's opt-out for an org-layer integration.
  final bool optedOut;

  IntegrationRow copyWith({IntegrationSummary? integration, bool? optedOut}) =>
      IntegrationRow(
        integration ?? this.integration,
        optedOut: optedOut ?? this.optedOut,
      );
}

class IntegrationListState {
  const IntegrationListState({
    this.rows = const [],
    this.busyIds = const {},
    this.message,
  });

  final List<IntegrationRow> rows;

  /// Integrations with a request in flight.
  final Set<String> busyIds;

  /// One-shot notice for the last action; cleared by
  /// [IntegrationListViewModel.clearMessage].
  final String? message;

  IntegrationListState copyWith({
    List<IntegrationRow>? rows,
    Set<String>? busyIds,
    Object? message = _keep,
  }) => IntegrationListState(
    rows: rows ?? this.rows,
    busyIds: busyIds ?? this.busyIds,
    message: identical(message, _keep) ? this.message : message as String?,
  );
}

const Object _keep = Object();

/// Loads and edits the integrations of one [IntegrationListScope].
class IntegrationListViewModel extends AsyncNotifier<IntegrationListState> {
  IntegrationListViewModel(this.scope);

  final IntegrationListScope scope;

  @override
  Future<IntegrationListState> build() async {
    final api = ref.watch(apiClientProvider);
    if (api == null) return const IntegrationListState();
    return IntegrationListState(rows: await _fetch());
  }

  /// Reloads the list.
  Future<void> refresh() async {
    final current = state.value ?? const IntegrationListState();
    state = AsyncData(current.copyWith(rows: await _fetch()));
  }

  /// Enables or disables [id].
  Future<void> setEnabled(String id, bool enabled) => _act(id, () async {
    final response = await ref
        .read(apiClientProvider)!
        .integrations
        .updateIntegration(
          id: id,
          updateIntegrationRequest: UpdateIntegrationRequest(
            (b) => b..enabled = enabled,
          ),
        );
    final detail = response.data!;
    _replace(id, (row) => row.copyWith(integration: _summaryOf(detail)));
  });

  /// Removes [id] and its credentials.
  Future<void> delete(String id) => _act(id, () async {
    await ref.read(apiClientProvider)!.integrations.deleteIntegration(id: id);
    final current = state.requireValue;
    state = AsyncData(
      current.copyWith(
        rows: [
          for (final row in current.rows)
            if (row.integration.id != id) row,
        ],
      ),
    );
  });

  /// Opts the space out of the org-layer integration [id], or back in.
  Future<void> setOptedOut(String id, bool optOut) => _act(id, () async {
    final spaceId = scope.spaceId!;
    final integrations = ref.read(apiClientProvider)!.integrations;
    if (optOut) {
      await integrations.putSpaceAccess(
        id: id,
        spaceId: spaceId,
        setSpaceAccessRequest: SetSpaceAccessRequest(
          (b) => b..mode = SpaceAccessMode.optOut,
        ),
      );
    } else {
      await integrations.deleteSpaceAccess(
        id: id,
        spaceId: spaceId,
        mode: SpaceAccessMode.optOut,
      );
    }
    _replace(id, (row) => row.copyWith(optedOut: optOut));
  });

  void clearMessage() {
    final current = state.value;
    if (current != null && current.message != null) {
      state = AsyncData(current.copyWith(message: null));
    }
  }

  Future<void> _act(String id, Future<void> Function() action) async {
    _setBusy(id, busy: true);
    try {
      await action();
    } catch (error) {
      await _report(error);
    }
    if (ref.mounted) _setBusy(id, busy: false);
  }

  Future<void> _report(Object error) async {
    final failure = describeIntegrationFailure(error);
    final stale =
        failure.kind == IntegrationFailureKind.conflict ||
        failure.kind == IntegrationFailureKind.notFound;
    var rows = state.requireValue.rows;
    if (stale) {
      try {
        rows = await _fetch();
      } catch (_) {
        // Keep the rows on screen; the message below still explains.
      }
    }
    if (!ref.mounted) return;
    state = AsyncData(
      state.requireValue.copyWith(
        rows: rows,
        message: stale
            ? '${failure.message} The list was refreshed.'
            : failure.message,
      ),
    );
  }

  void _setBusy(String id, {required bool busy}) {
    final current = state.requireValue;
    state = AsyncData(
      current.copyWith(
        busyIds: busy
            ? {...current.busyIds, id}
            : ({...current.busyIds}..remove(id)),
      ),
    );
  }

  void _replace(String id, IntegrationRow Function(IntegrationRow) change) {
    final current = state.requireValue;
    state = AsyncData(
      current.copyWith(
        rows: [
          for (final row in current.rows)
            if (row.integration.id == id) change(row) else row,
        ],
      ),
    );
  }

  Future<List<IntegrationRow>> _fetch() async {
    final integrations = ref.read(apiClientProvider)!.integrations;
    switch (scope.kind) {
      case IntegrationScopeKind.org:
        final org = await _list(integrations, layer: IntegrationLayer.org);
        return [for (final item in org) IntegrationRow(item)];
      case IntegrationScopeKind.connections:
        final userId = await ref.read(currentUserIdProvider.future);
        final all = await _list(integrations);
        return [
          for (final item in all)
            if (_isConnectionOf(item, userId)) IntegrationRow(item),
        ];
      case IntegrationScopeKind.space:
        return _fetchSpace(integrations, scope.spaceId!);
    }
  }

  /// Own and offered integrations, then the org ones the space opted out of.
  ///
  /// The server leaves opted-out integrations out of a space's list, so they
  /// are found by diffing against every org integration the caller can see
  /// and confirming the opt-out on the integration itself.
  Future<List<IntegrationRow>> _fetchSpace(
    IntegrationsApi integrations,
    String spaceId,
  ) async {
    final results = await Future.wait([
      _list(integrations, spaceId: spaceId),
      _list(
        integrations,
        layer: IntegrationLayer.org,
      ).catchError((_) => <IntegrationSummary>[]),
    ]);
    final offered = results[0];
    final offeredIds = {for (final item in offered) item.id};
    final hidden = results[1].where((item) => !offeredIds.contains(item.id));

    final optedOut = await Future.wait([
      for (final item in hidden)
        () async {
          try {
            final detail = (await integrations.getIntegration(id: item.id))
                .data!;
            final isOut = detail.spaceAccess.any(
              (entry) =>
                  entry.spaceId == spaceId &&
                  entry.mode == SpaceAccessMode.optOut,
            );
            return isOut ? IntegrationRow(item, optedOut: true) : null;
          } catch (_) {
            return null;
          }
        }(),
    ]);
    return [
      for (final item in offered) IntegrationRow(item),
      ...optedOut.nonNulls,
    ];
  }

  Future<List<IntegrationSummary>> _list(
    IntegrationsApi integrations, {
    IntegrationLayer? layer,
    String? spaceId,
  }) async {
    final response = await integrations.listIntegrations(
      layer: layer,
      spaceId: spaceId,
    );
    return response.data?.items.toList() ?? const [];
  }

  /// A personal integration of [userId], or an org/space one that needs the
  /// caller's own credential.
  bool _isConnectionOf(IntegrationSummary item, String userId) =>
      item.layer == IntegrationLayer.user
      ? item.ownerUserId == userId
      : item.credentialMode == CredentialMode.perUser;
}

IntegrationSummary _summaryOf(IntegrationDetail detail) => IntegrationSummary(
  (b) => b
    ..id = detail.id
    ..layer = detail.layer
    ..slug = detail.slug
    ..name = detail.name
    ..connectUrl = detail.connectUrl
    ..catalogEntryId = detail.catalogEntryId
    ..authKind = detail.authKind
    ..credentialMode = detail.credentialMode
    ..trust = detail.trust
    ..enabled = detail.enabled
    ..status = detail.status
    ..hasCredential = detail.hasCredential
    ..ownerUserId = detail.ownerUserId
    ..spaceId = detail.spaceId
    ..createdAt = detail.createdAt
    ..updatedAt = detail.updatedAt,
);

final integrationListViewModelProvider = AsyncNotifierProvider.autoDispose
    .family<
      IntegrationListViewModel,
      IntegrationListState,
      IntegrationListScope
    >(
      IntegrationListViewModel.new,
      // A failed load shows an error with a retry button, not a silent
      // backoff loop.
      retry: (_, _) => null,
    );
