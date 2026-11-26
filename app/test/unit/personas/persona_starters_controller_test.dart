import 'package:assistant_app/features/connection/connection_provider.dart';
import 'package:assistant_app/features/personas/persona_detail_provider.dart';
import 'package:assistant_app/features/personas/persona_starters_controller.dart';
import 'package:assistant_app/features/personas/personas_provider.dart';
import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

import '../../support/fake_personas_api.dart';

ProviderContainer _container(FakePersonasApi api) {
  final container = ProviderContainer(
    overrides: [
      apiClientProvider.overrideWithValue(FakePersonasApiClient(api)),
    ],
  );
  addTearDown(container.dispose);
  return container;
}

// ---------------------------------------------------------------------------
// Tests

void main() {
  group('personaStartersProvider', () {
    test('is loading, then success, from personaDetailProvider', () async {
      final api = FakePersonasApi(starters: ['Plan my week']);
      final container = _container(api);
      final sub = container.listen(personaStartersProvider('p1'), (_, _) {});
      addTearDown(sub.close);

      expect(container.read(personaStartersProvider('p1')).isLoading, isTrue);

      await container.read(personaDetailProvider('p1').future);
      final view = container.read(personaStartersProvider('p1')).requireValue;
      expect(view.prompts, ['Plan my week']);
      expect(view.canEdit, isTrue);
      expect(api.getCalls, 1, reason: 'no second getPersona call');
    });

    test('surfaces a fetch error from personaDetailProvider', () async {
      final api = FakePersonasApi();
      final container = ProviderContainer(
        overrides: [
          apiClientProvider.overrideWithValue(FakePersonasApiClient(api)),
          personaDetailProvider('p1')
              .overrideWith(() => _ErrorDetailNotifier()),
        ],
      );
      addTearDown(container.dispose);
      final sub = container.listen(personaStartersProvider('p1'), (_, _) {});
      addTearDown(sub.close);

      await container.read(personaDetailProvider('p1').future);
      expect(container.read(personaStartersProvider('p1')).hasError, isTrue);
    });

    test('exposes canEdit false from the detail', () async {
      final container = _container(
        FakePersonasApi(starters: ['A'], canEdit: false),
      );
      final sub = container.listen(personaStartersProvider('p1'), (_, _) {});
      addTearDown(sub.close);

      await container.read(personaDetailProvider('p1').future);
      final view = container.read(personaStartersProvider('p1')).requireValue;
      expect(view.canEdit, isFalse);
      expect(view.prompts, ['A']);
    });
  });

  group('PersonaStartersController.save', () {
    test('calls updatePersona and invalidates both providers', () async {
      final api = FakePersonasApi(starters: ['Old']);
      final container = _container(api);
      final detailSub = container.listen(
        personaStartersProvider('p1'),
        (_, _) {},
      );
      addTearDown(detailSub.close);
      final seen = <List<String>>[];
      final activeSub = container.listen(
        activePersonaStarterPromptsProvider,
        (_, next) => seen.add(next),
        fireImmediately: true,
      );
      addTearDown(activeSub.close);
      await container.read(personasProvider.future);
      await container.read(personaDetailProvider('p1').future);
      expect(container.read(activePersonaStarterPromptsProvider), ['Old']);
      final callsBefore = api.getCalls;

      await container.read(personaStartersControllerProvider('p1')).save([
        '  Plan my week ',
        'Second',
      ]);

      expect(api.updates.single, ['Plan my week', 'Second']);
      await container.read(personaDetailProvider('p1').future);
      expect(api.getCalls, greaterThan(callsBefore), reason: 'detail refetch');
      expect(
        container.read(personaStartersProvider('p1')).requireValue.prompts,
        ['Plan my week', 'Second'],
      );
      expect(container.read(activePersonaStarterPromptsProvider), [
        'Plan my week',
        'Second',
      ]);
    });

    test('is not offered without edit permission', () async {
      final api = FakePersonasApi(starters: ['A'], canEdit: false);
      final container = _container(api);
      final sub = container.listen(personaStartersProvider('p1'), (_, _) {});
      addTearDown(sub.close);
      await container.read(personaDetailProvider('p1').future);

      await expectLater(
        container.read(personaStartersControllerProvider('p1')).save(['B']),
        throwsA(isA<StarterPromptsException>()),
      );
      expect(api.updates, isEmpty);
    });

    test('propagates an API failure and keeps the detail', () async {
      final api = FakePersonasApi(starters: ['A'])
        ..updateError = DioException(
          requestOptions: RequestOptions(path: '/api/personas/p1'),
        );
      final container = _container(api);
      final sub = container.listen(personaStartersProvider('p1'), (_, _) {});
      addTearDown(sub.close);
      await container.read(personaDetailProvider('p1').future);

      await expectLater(
        container.read(personaStartersControllerProvider('p1')).save(['B']),
        throwsA(isA<DioException>()),
      );
      expect(
        container.read(personaStartersProvider('p1')).requireValue.prompts,
        ['A'],
      );
    });
  });

  group('validateStarterPrompts', () {
    test('accepts up to 4 non-blank prompts of at most 200 runes', () {
      expect(validateStarterPrompts([]), isNull);
      expect(validateStarterPrompts(['a', 'b', 'c', 'd']), isNull);
      expect(validateStarterPrompts(['😀' * 200]), isNull);
    });

    test('rejects a 5th prompt', () {
      expect(validateStarterPrompts(['a', 'b', 'c', 'd', 'e']), isNotNull);
    });

    test('rejects blank prompts', () {
      expect(validateStarterPrompts(['a', '   ']), isNotNull);
      expect(validateStarterPrompts(['']), isNotNull);
    });

    test('rejects prompts over 200 runes after trimming', () {
      expect(validateStarterPrompts(['x' * 201]), isNotNull);
      expect(validateStarterPrompts(['  ${'x' * 200}  ']), isNull);
      expect(validateStarterPrompts(['😀' * 201]), isNotNull);
    });
  });

  group('activePersonaStarterPromptsProvider', () {
    test('is empty while the persona is loading and without an api', () {
      final container = ProviderContainer(
        overrides: [apiClientProvider.overrideWithValue(null)],
      );
      addTearDown(container.dispose);
      expect(container.read(activePersonaStarterPromptsProvider), isEmpty);
    });
  });
}

class _ErrorDetailNotifier extends PersonaDetailNotifier {
  _ErrorDetailNotifier() : super('p1');

  @override
  Future<PersonaDetailState> build() async =>
      const PersonaDetailState(error: 'boom');
}
