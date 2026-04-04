import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../api/client.dart';
import '../../api/endpoints/skills.dart';
import '../../api/models/skill.dart';
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

  final List<Skill> skills;

  /// The persona ID whose skills are shown.
  final String? personaId;

  final bool isLoading;
  final String? error;

  SkillsState copyWith({
    List<Skill>? skills,
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
class SkillsNotifier extends AutoDisposeAsyncNotifier<SkillsState> {
  @override
  Future<SkillsState> build() async {
    // Refresh skills whenever the active persona changes.
    ref.listen(personasProvider, (_, next) {
      final newPersonaId = next.valueOrNull?.activePersona?.id;
      if (newPersonaId != null) {
        _reload(newPersonaId);
      }
    });

    return _fetchSkills();
  }

  SkillsEndpoint? get _endpoint {
    final profile = ref.read(activeProfileProvider);
    if (profile == null) return null;
    final client = AssistantClient(
      baseUrl: profile.baseUrl,
      token: profile.token,
    );
    return SkillsEndpoint(client);
  }

  Future<SkillsState> _fetchSkills() async {
    final endpoint = _endpoint;
    if (endpoint == null) return const SkillsState();

    // Use the active persona ID.
    final personaId =
        ref.read(personasProvider).valueOrNull?.activePersona?.id ?? 'default';

    try {
      final skills = await endpoint.listForPersona(personaId);
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
}

/// Provider for [SkillsNotifier].
final skillsProvider =
    AsyncNotifierProvider.autoDispose<SkillsNotifier, SkillsState>(
  SkillsNotifier.new,
);
