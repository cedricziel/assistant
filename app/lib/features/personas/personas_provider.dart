import 'package:assistant_api/assistant_api.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../api/api_client.dart';
import '../connection/connection_provider.dart';

/// State for the personas feature.
class PersonasState {
  const PersonasState({
    this.personas = const [],
    this.activePersona,
    this.isLoading = false,
    this.error,
  });

  final List<PersonaSummary> personas;
  final PersonaSummary? activePersona;
  final bool isLoading;
  final String? error;

  PersonasState copyWith({
    List<PersonaSummary>? personas,
    PersonaSummary? activePersona,
    bool? isLoading,
    String? error,
    bool clearError = false,
  }) {
    return PersonasState(
      personas: personas ?? this.personas,
      activePersona: activePersona ?? this.activePersona,
      isLoading: isLoading ?? this.isLoading,
      error: clearError ? null : (error ?? this.error),
    );
  }
}

/// Notifier for persona list and active persona management.
class PersonasNotifier extends AutoDisposeAsyncNotifier<PersonasState> {
  @override
  Future<PersonasState> build() async {
    return _fetchPersonas();
  }

  ApiClient? get _api {
    final profile = ref.read(activeProfileProvider);
    if (profile == null) return null;
    return ApiClient(baseUrl: profile.baseUrl, token: profile.token);
  }

  Future<PersonasState> _fetchPersonas() async {
    final api = _api;
    if (api == null) return const PersonasState();

    try {
      final response = await api.personas.listPersonas();
      final personas = response.data!.toList();
      final defaultPersona =
          personas.where((p) => p.isDefault).firstOrNull ?? personas.firstOrNull;
      return PersonasState(
        personas: personas,
        activePersona: defaultPersona,
      );
    } catch (e) {
      return PersonasState(error: e.toString());
    }
  }

  /// Refresh the personas list from the server.
  Future<void> refresh() async {
    state = const AsyncLoading();
    state = AsyncData(await _fetchPersonas());
  }

  /// Switch the active persona on the server and update local state.
  Future<void> switchPersona(String id) async {
    final api = _api;
    if (api == null) return;

    final current = state.valueOrNull ?? const PersonasState();
    state = AsyncData(current.copyWith(isLoading: true, clearError: true));

    try {
      final response = await api.personas.setActivePersona(
        setActivePersonaRequest: SetActivePersonaRequest((b) => b.id = id),
      );
      final updated = response.data!;
      state = AsyncData(
        current.copyWith(
          activePersona: updated,
          isLoading: false,
        ),
      );
    } catch (e) {
      state = AsyncData(
        current.copyWith(
          isLoading: false,
          error: e.toString(),
        ),
      );
    }
  }
}

/// Provider for [PersonasNotifier].
final personasProvider =
    AsyncNotifierProvider.autoDispose<PersonasNotifier, PersonasState>(
  PersonasNotifier.new,
);
