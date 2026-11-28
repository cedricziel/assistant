/// A user action on an A2UI surface, sent on the post-turn route.
///
/// Mirrors the server's `SurfaceActionRequest`: the action the surface
/// declared, the component that raised it and the surface's data model when it
/// sends one. The visible user message is built by the server.
class SurfaceActionSubmission {
  const SurfaceActionSubmission({
    required this.surfaceId,
    required this.name,
    required this.sourceComponentId,
    required this.context,
    this.dataModel,
  });

  final String surfaceId;
  final String name;
  final String sourceComponentId;
  final Map<String, Object?> context;
  final Map<String, Object?>? dataModel;

  Map<String, Object?> toJson() => {
    'surface_id': surfaceId,
    'name': name,
    'source_component_id': sourceComponentId,
    'context': context,
    'data_model': ?dataModel,
  };
}
