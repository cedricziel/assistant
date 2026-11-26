import 'package:assistant_api/assistant_api.dart';
import 'package:built_collection/built_collection.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../connection/connection_provider.dart';
import 'persona_detail_provider.dart';
import 'personas_provider.dart';

/// Most starter prompts a Persona may have.
const int kMaxStarterPrompts = 4;

/// Longest starter prompt, counted in Unicode scalar values after trimming.
const int kMaxStarterPromptLength = 200;

/// Thrown by [PersonaStartersController.save] when the save is not allowed or
/// the prompts violate the limits.
class StarterPromptsException implements Exception {
  const StarterPromptsException(this.message);

  final String message;

  @override
  String toString() => message;
}

/// Checks [prompts] against the same limits as the API.
///
/// Returns a user-facing message, or `null` when the list is valid.
String? validateStarterPrompts(List<String> prompts) {
  if (prompts.length > kMaxStarterPrompts) {
    return 'At most $kMaxStarterPrompts starter prompts are allowed.';
  }
  for (final prompt in prompts) {
    final trimmed = prompt.trim();
    if (trimmed.isEmpty) return 'Starter prompts cannot be blank.';
    if (trimmed.runes.length > kMaxStarterPromptLength) {
      return 'Starter prompts are limited to $kMaxStarterPromptLength '
          'characters.';
    }
  }
  return null;
}

/// What the starter prompts UI needs from a Persona detail.
class PersonaStartersView {
  const PersonaStartersView({required this.prompts, required this.canEdit});

  final List<String> prompts;
  final bool canEdit;
}

/// Starter prompts of one Persona, derived from [personaDetailProvider] so
/// there is no second `getPersona` call. Loading and error states pass through.
final personaStartersProvider = Provider.autoDispose
    .family<AsyncValue<PersonaStartersView>, String>((ref, id) {
      return ref.watch(personaDetailProvider(id)).whenData((state) {
        final detail = state.detail;
        if (detail == null) {
          throw StateError(state.error ?? 'Persona not found');
        }
        return PersonaStartersView(
          prompts: detail.starterPrompts.toList(),
          canEdit: detail.canEditStarterPrompts,
        );
      });
    });

/// Starter prompts of the active Persona; empty while loading, on error, or
/// when the Persona has none. There is no org-level fallback.
final activePersonaStarterPromptsProvider = Provider.autoDispose<List<String>>((
  ref,
) {
  final active = ref.watch(
    personasProvider.select((s) => s.value?.activePersona?.id),
  );
  if (active == null) return const [];
  return ref.watch(personaStartersProvider(active)).value?.prompts ?? const [];
});

/// Saves starter prompts for one Persona.
class PersonaStartersController {
  PersonaStartersController(this._ref, this._personaId);

  final Ref _ref;
  final String _personaId;

  /// Validates, trims and stores [prompts], then refreshes every view of them.
  ///
  /// Throws [StarterPromptsException] when the caller may not edit or the
  /// prompts are invalid; API failures propagate unchanged.
  Future<void> save(List<String> prompts) async {
    final view = _ref.read(personaStartersProvider(_personaId)).value;
    if (view == null || !view.canEdit) {
      throw const StarterPromptsException(
        'You do not have permission to edit starter prompts.',
      );
    }
    final error = validateStarterPrompts(prompts);
    if (error != null) throw StarterPromptsException(error);
    final api = _ref.read(apiClientProvider);
    if (api == null) throw const StarterPromptsException('Not connected.');

    await api.personas.updatePersona(
      id: _personaId,
      updatePersonaRequest: UpdatePersonaRequest(
        (b) => b
          ..starterPrompts = ListBuilder<String>(prompts.map((p) => p.trim())),
      ),
    );
    _ref.invalidate(personaDetailProvider(_personaId));
    _ref.invalidate(activePersonaStarterPromptsProvider);
  }
}

/// Not autoDispose: [PersonaStartersController.save] outlives the read that
/// obtains the controller, and the provider watches nothing, so it never
/// rebuilds under the controller's [Ref].
final personaStartersControllerProvider =
    Provider.family<PersonaStartersController, String>(
      (ref, id) => PersonaStartersController(ref, id),
    );
