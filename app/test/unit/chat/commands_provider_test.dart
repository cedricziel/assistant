import 'dart:async';

import 'package:assistant_api/assistant_api.dart';
import 'package:assistant_app/api/api_client.dart';
import 'package:assistant_app/features/chat/commands_provider.dart';
import 'package:assistant_app/features/connection/connection_provider.dart';
import 'package:built_collection/built_collection.dart';
import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

// -- Fake CommandsApi ---------------------------------------------------------

class _FakeCommandsApi extends CommandsApi {
  _FakeCommandsApi({this.commands = const [], this.error})
    : super(Dio(), standardSerializers);

  final List<CommandDefResponse> commands;
  final Exception? error;

  @override
  Future<Response<BuiltList<CommandDefResponse>>> listCommands({
    CancelToken? cancelToken,
    Map<String, dynamic>? headers,
    Map<String, dynamic>? extra,
    ValidateStatus? validateStatus,
    ProgressCallback? onSendProgress,
    ProgressCallback? onReceiveProgress,
  }) async {
    if (error != null) throw error!;
    return Response(
      data: BuiltList.of(commands),
      requestOptions: RequestOptions(path: '/api/commands'),
      statusCode: 200,
    );
  }
}

class _FakeApiClient extends ApiClient {
  _FakeApiClient({required this.fakeCommands})
    : super(baseUrl: 'http://fake.test', token: 'test-token');

  final _FakeCommandsApi fakeCommands;

  @override
  CommandsApi get commands => fakeCommands;
}

// -- Helpers ------------------------------------------------------------------

CommandDefResponse _makeDef({
  required String name,
  String description = '',
  String category = 'info',
  List<CommandArgResponse> args = const [],
}) {
  return CommandDefResponse(
    (b) => b
      ..name = name
      ..description = description
      ..category = category
      ..args = ListBuilder<CommandArgResponse>(args),
  );
}

CommandArgResponse _makeArg({
  required String name,
  bool required = false,
  String? completionsEndpoint,
}) {
  return CommandArgResponse(
    (b) => b
      ..name = name
      ..required_ = required
      ..completionsEndpoint = completionsEndpoint,
  );
}

ProviderContainer _makeContainer({ApiClient? client}) {
  return ProviderContainer(
    overrides: [apiClientProvider.overrideWithValue(client)],
  );
}

// -- Tests --------------------------------------------------------------------

void main() {
  group('commandsProvider', () {
    test('returns empty list when apiClient is null', () async {
      final container = _makeContainer(client: null);
      addTearDown(container.dispose);

      final result = await container.read(commandsProvider.future);

      expect(result, isEmpty);
    });

    test('maps API response to CommandEntry list', () async {
      final fakeApi = _FakeCommandsApi(
        commands: [
          _makeDef(name: 'help', description: 'Show help', category: 'info'),
          _makeDef(
            name: 'model',
            description: 'Set model',
            category: 'config',
            args: [
              _makeArg(
                name: 'model_name',
                required: false,
                completionsEndpoint: '/api/models',
              ),
            ],
          ),
        ],
      );
      final client = _FakeApiClient(fakeCommands: fakeApi);
      final container = _makeContainer(client: client);
      addTearDown(container.dispose);

      final result = await container.read(commandsProvider.future);

      expect(result, hasLength(2));

      expect(result[0].name, 'help');
      expect(result[0].description, 'Show help');
      expect(result[0].category, 'info');
      expect(result[0].hasArgs, isFalse);

      expect(result[1].name, 'model');
      expect(result[1].description, 'Set model');
      expect(result[1].category, 'config');
      expect(result[1].hasArgs, isTrue);
      expect(result[1].args, hasLength(1));
      expect(result[1].args[0].name, 'model_name');
      expect(result[1].args[0].required, isFalse);
      expect(result[1].args[0].completionsEndpoint, '/api/models');
    });

    test('surfaces error when API throws', () async {
      final fakeApi = _FakeCommandsApi(
        error: DioException(
          requestOptions: RequestOptions(path: '/api/commands'),
          type: DioExceptionType.connectionError,
        ),
      );
      final client = _FakeApiClient(fakeCommands: fakeApi);
      final container = _makeContainer(client: client);
      addTearDown(container.dispose);

      // Listen and wait for the provider to settle into an error state.
      final completer = Completer<AsyncValue<List<CommandEntry>>>();
      final sub = container.listen(commandsProvider, (prev, next) {
        if (next.hasError && !completer.isCompleted) {
          completer.complete(next);
        }
      });
      addTearDown(sub.close);

      final value = await completer.future;
      expect(value.hasError, isTrue);
      expect(value.error, isA<DioException>());
    });

    test('maps command with no args correctly', () async {
      final fakeApi = _FakeCommandsApi(
        commands: [
          _makeDef(name: 'stop', description: 'Stop turn', category: 'session'),
        ],
      );
      final client = _FakeApiClient(fakeCommands: fakeApi);
      final container = _makeContainer(client: client);
      addTearDown(container.dispose);

      final result = await container.read(commandsProvider.future);

      expect(result, hasLength(1));
      expect(result[0].name, 'stop');
      expect(result[0].args, isEmpty);
      expect(result[0].hasArgs, isFalse);
    });
  });
}
