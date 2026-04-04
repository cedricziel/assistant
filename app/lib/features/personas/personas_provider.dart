import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../api/client.dart';
import '../../api/endpoints/personas.dart';
import '../../api/models/persona.dart';
import '../connection/connection_provider.dart';

/// State for the personas feature.
class PersonasState {
  const PersonasState({
    this.personas = const [],
    this.activePersona,
    this.isLoading = false,
    this.error,
  });

  final List<Persona> personas;
  final Persona? activePersona;
  final bool isLoading;
  final String? error;

  PersonasState copyWith({
    List<Persona>? personas,
    Persona? activePersona,
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

  PersonasEndpoint? get _endpoint {
    final profile = ref.read(activeProfileProvider);
    if (profile == null) return null;
    final client = AssistantClient(
      baseUrl: profile.baseUrl,
      token: profile.token,
    );
    return PersonasEndpoint(client);
  }

  Future<PersonasState> _fetchPersonas() async {
    final endpoint = _endpoint;
    if (endpoint == null) return const PersonasState();

    try {
      final personas = await endpoint.list();
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
    final endpoint = _endpoint;
    if (endpoint == null) return;

    final current = state.valueOrNull ?? const PersonasState();
    state = AsyncData(current.copyWith(isLoading: true, clearError: true));

    try {
      final updated = await endpoint.setActive(id);
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
