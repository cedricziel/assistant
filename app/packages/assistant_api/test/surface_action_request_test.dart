import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

// tests for SurfaceActionRequest
void main() {
  final instance = SurfaceActionRequestBuilder();
  // TODO add properties to the builder and call build()

  group(SurfaceActionRequest, () {
    // The action's context payload.
    // JsonObject context
    test('to test the property `context`', () async {
      // TODO
    });

    // The surface's data model at the time of the action.
    // JsonObject dataModel
    test('to test the property `dataModel`', () async {
      // TODO
    });

    // The action name, which the surface must declare on a Button.
    // String name
    test('to test the property `name`', () async {
      // TODO
    });

    // Id of the Button component that fired the action.
    // String sourceComponentId
    test('to test the property `sourceComponentId`', () async {
      // TODO
    });

    // The surface the action belongs to.
    // String surfaceId
    test('to test the property `surfaceId`', () async {
      // TODO
    });
  });
}
