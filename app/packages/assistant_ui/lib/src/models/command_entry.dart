/// A single command argument definition.
class CommandArg {
  const CommandArg({
    required this.name,
    required this.required,
    this.completionsEndpoint,
  });

  final String name;
  final bool required;
  final String? completionsEndpoint;
}

/// A command definition for autocomplete.
class CommandEntry {
  const CommandEntry({
    required this.name,
    required this.description,
    required this.category,
    this.args = const [],
  });

  final String name;
  final String description;
  final String category;
  final List<CommandArg> args;

  /// Whether this command has any arguments.
  bool get hasArgs => args.isNotEmpty;
}
