import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

// tests for Artifact
void main() {
  final instance = ArtifactBuilder();
  // TODO add properties to the builder and call build()

  group(Artifact, () {
    // Unique identifier (UUID) for the artifact, unique within a task.
    // String artifactId
    test('to test the property `artifactId`', () async {
      // TODO
    });

    // A human-readable description of the artifact.
    // String description
    test('to test the property `description`', () async {
      // TODO
    });

    // The URIs of extensions present or contributing to this artifact.
    // BuiltList<String> extensions
    test('to test the property `extensions`', () async {
      // TODO
    });

    // Metadata included with the artifact.
    // BuiltMap<String, JsonObject> metadata
    test('to test the property `metadata`', () async {
      // TODO
    });

    // A human-readable name for the artifact.
    // String name
    test('to test the property `name`', () async {
      // TODO
    });

    // The content of the artifact. Must contain at least one part.
    // BuiltList<ModelPart> parts
    test('to test the property `parts`', () async {
      // TODO
    });
  });
}
