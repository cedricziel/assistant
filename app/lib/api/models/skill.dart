/// A skill available to a persona.
///
/// Read-only from the app's perspective (US5 is discovery only).
class Skill {
  const Skill({
    required this.name,
    required this.description,
    required this.enabled,
  });

  final String name;

  /// Short description of what this skill does (may be empty).
  final String description;

  /// Whether this skill is enabled for the active persona.
  final bool enabled;

  factory Skill.fromJson(Map<String, dynamic> json) {
    return Skill(
      name: json['name'] as String,
      description: json['description'] as String? ?? '',
      enabled: json['enabled'] as bool? ?? false,
    );
  }
}
