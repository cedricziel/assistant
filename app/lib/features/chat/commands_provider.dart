import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../connection/connection_provider.dart';

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

/// Fetches and caches the list of available slash commands.
final commandsProvider = FutureProvider<List<CommandEntry>>((ref) async {
  final api = ref.watch(apiClientProvider);
  if (api == null) return [];

  final response = await api.commands.listCommands();
  final defs = response.data;
  if (defs == null) return [];

  return defs
      .map(
        (def) => CommandEntry(
          name: def.name,
          description: def.description,
          category: def.category,
          args: def.args
              .map(
                (a) => CommandArg(
                  name: a.name,
                  required: a.required_,
                  completionsEndpoint: a.completionsEndpoint,
                ),
              )
              .toList(),
        ),
      )
      .toList();
});
