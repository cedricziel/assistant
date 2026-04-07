import 'package:assistant_api/assistant_api.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../api/api_client.dart';
import '../connection/connection_provider.dart';

/// State for the persona detail screen.
class PersonaDetailState {
  const PersonaDetailState({
    this.detail,
    this.isLoading = false,
    this.error,
  });

  final PersonaDetail? detail;
  final bool isLoading;
  final String? error;
}

/// Notifier for a single persona detail.
class PersonaDetailNotifier extends AsyncNotifier<PersonaDetailState> {
  PersonaDetailNotifier(this._personaId);

  final String _personaId;

  @override
  Future<PersonaDetailState> build() async {
    return _fetchDetail(_personaId);
  }

  ApiClient? get _api {
    final profile = ref.read(activeProfileProvider);
    if (profile == null) return null;
    return ApiClient(baseUrl: profile.baseUrl, token: profile.token);
  }

  Future<PersonaDetailState> _fetchDetail(String personaId) async {
    final api = _api;
    if (api == null) return const PersonaDetailState();

    try {
      final response = await api.personas.getPersona(id: personaId);
      return PersonaDetailState(detail: response.data!);
    } catch (e) {
      return PersonaDetailState(error: e.toString());
    }
  }

  Future<void> refresh() async {
    state = const AsyncLoading();
    state = AsyncData(await _fetchDetail(_personaId));
  }
}

/// Family provider for [PersonaDetailNotifier].
final personaDetailProvider = AsyncNotifierProvider.autoDispose
    .family<PersonaDetailNotifier, PersonaDetailState, String>(
  (arg) => PersonaDetailNotifier(arg),
);
