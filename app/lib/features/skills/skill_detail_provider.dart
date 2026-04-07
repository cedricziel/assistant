import 'package:assistant_api/assistant_api.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../api/api_client.dart';
import '../connection/connection_provider.dart';

/// State for the skill detail screen.
class SkillDetailState {
  const SkillDetailState({
    this.skill,
    this.isLoading = false,
    this.error,
  });

  final SkillDetail? skill;
  final bool isLoading;
  final String? error;
}

/// Notifier for a single skill identified by name.
class SkillDetailNotifier extends AsyncNotifier<SkillDetailState> {
  SkillDetailNotifier(this._skillName);

  final String _skillName;

  @override
  Future<SkillDetailState> build() async {
    return _fetch(_skillName);
  }

  ApiClient? get _api {
    final profile = ref.read(activeProfileProvider);
    if (profile == null) return null;
    return ApiClient(baseUrl: profile.baseUrl, token: profile.token);
  }

  Future<SkillDetailState> _fetch(String name) async {
    final api = _api;
    if (api == null) return const SkillDetailState();
    try {
      final response = await api.skills.getSkill(name: name);
      return SkillDetailState(skill: response.data);
    } catch (e) {
      return SkillDetailState(error: e.toString());
    }
  }

  Future<void> refresh() async {
    state = const AsyncLoading();
    state = AsyncData(await _fetch(_skillName));
  }
}

/// Family provider for [SkillDetailNotifier], parameterised by skill name.
final skillDetailProvider = AsyncNotifierProvider.autoDispose
    .family<SkillDetailNotifier, SkillDetailState, String>(
  (arg) => SkillDetailNotifier(arg),
);
