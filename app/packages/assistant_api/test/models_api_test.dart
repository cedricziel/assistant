import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

/// tests for ModelsApi
void main() {
  final instance = AssistantApi().getModelsApi();

  group(ModelsApi, () {
    // `GET /api/models` — list the models a reply may be regenerated with.
    //
    //Future<BuiltList<ModelSummary>> listModels({ String xSpaceId }) async
    test('test listModels', () async {
      // TODO
    });
  });
}
