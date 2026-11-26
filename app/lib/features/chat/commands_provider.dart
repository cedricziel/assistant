import 'package:assistant_ui/models.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../connection/connection_provider.dart';

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
