/// A backend-defined persona (agent configuration).
///
/// The app can display and select personas but does not create or edit them.
class Persona {
  const Persona({
    required this.id,
    required this.name,
    required this.description,
    required this.isDefault,
  });

  final String id;
  final String name;

  /// Short description of the persona's capabilities (may be empty).
  final String description;

  /// Whether this is the server's default persona.
  final bool isDefault;

  factory Persona.fromJson(Map<String, dynamic> json) {
    return Persona(
      id: json['id'] as String,
      name: json['name'] as String,
      description: json['description'] as String? ?? '',
      isDefault: json['is_default'] as bool? ?? false,
    );
  }

  @override
  bool operator ==(Object other) =>
      identical(this, other) || other is Persona && id == other.id;

  @override
  int get hashCode => id.hashCode;
}
