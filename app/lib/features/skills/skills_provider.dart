import 'package:assistant_api/assistant_api.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../api/api_client.dart';
import '../connection/connection_provider.dart';
import '../personas/personas_provider.dart';

/// State for the skills screen.
class SkillsState {
  const SkillsState({
    this.skills = const [],
    this.personaId,
    this.isLoading = false,
    this.error,
  });

  final List<SkillEntryResponse> skills;

  /// The persona ID whose skills are shown.
  final String? personaId;

  final bool isLoading;
  final String? error;

  SkillsState copyWith({
    List<SkillEntryResponse>? skills,
    String? personaId,
    bool? isLoading,
    String? error,
    bool clearError = false,
  }) {
    return SkillsState(
      skills: skills ?? this.skills,
      personaId: personaId ?? this.personaId,
      isLoading: isLoading ?? this.isLoading,
      error: clearError ? null : (error ?? this.error),
    );
  }
}

/// Notifier for the skills list, bound to the active persona.
class SkillsNotifier extends AsyncNotifier<SkillsState> {
  @override
  Future<SkillsState> build() async {
    // Refresh skills whenever the active persona changes.
    ref.listen(personasProvider, (_, next) {
      final newPersonaId = next.value?.activePersona?.id;
      if (newPersonaId != null) {
        _reload(newPersonaId);
      }
    });

    return _fetchSkills();
  }

  ApiClient? get _api {
    final profile = ref.read(activeProfileProvider);
    if (profile == null) return null;
    return ApiClient(baseUrl: profile.baseUrl, token: profile.token);
  }

  Future<SkillsState> _fetchSkills() async {
    final api = _api;
    if (api == null) return const SkillsState();

    // Use the active persona ID.
    final personaId =
        ref.read(personasProvider).value?.activePersona?.id ?? 'default';

    try {
      final response = await api.skills.listPersonaSkills(personaId: personaId);
      final skills = response.data!.toList();
      return SkillsState(skills: skills, personaId: personaId);
    } catch (e) {
      return SkillsState(error: e.toString(), personaId: personaId);
    }
  }

  void _reload(String personaId) {
    state = const AsyncLoading();
    () async {
      state = AsyncData(await _fetchSkills());
    }();
  }

  Future<void> refresh() async {
    state = const AsyncLoading();
    state = AsyncData(await _fetchSkills());
  }

  /// Creates a new user skill. Returns an error string on failure, null on success.
  Future<String?> createSkill({
    required String name,
    required String description,
    required String body,
  }) async {
    final api = _api;
    if (api == null) return 'Not connected';
    try {
      final request = CreateSkillRequestBuilder()
        ..name = name
        ..description = description
        ..body = body;
      await api.skills.createSkill(createSkillRequest: request.build());
      await refresh();
      return null;
    } catch (e) {
      return e.toString();
    }
  }

  /// Updates an existing skill. Returns an error string on failure, null on success.
  Future<String?> updateSkill({
    required String name,
    required String description,
    required String body,
  }) async {
    final api = _api;
    if (api == null) return 'Not connected';
    try {
      final request = UpdateSkillRequestBuilder()
        ..description = description
        ..body = body;
      await api.skills.updateSkill(
        name: name,
        updateSkillRequest: request.build(),
      );
      await refresh();
      return null;
    } catch (e) {
      return e.toString();
    }
  }

  /// Deletes a skill by name. Returns an error string on failure, null on success.
  Future<String?> deleteSkill(String name) async {
    final api = _api;
    if (api == null) return 'Not connected';
    try {
      await api.skills.deleteSkill(name: name);
      await refresh();
      return null;
    } catch (e) {
      return e.toString();
    }
  }
}

/// Provider for [SkillsNotifier].
final skillsProvider =
    AsyncNotifierProvider.autoDispose<SkillsNotifier, SkillsState>(
  SkillsNotifier.new,
);
