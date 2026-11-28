import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../connection/connection_provider.dart';

/// Models a reply may be regenerated with, from `GET /api/models`.
///
/// The server lists the org's default model first, then the `[llm]
/// available_models` allowlist. Consumers read the value through `asData` and
/// hide the model menu when the list is loading, failed or holds fewer than
/// two entries, so a failed request never breaks the timeline.
final modelsProvider = FutureProvider<List<RegenerateModel>>((ref) async {
  final api = ref.watch(apiClientProvider);
  if (api == null) return const [];

  final response = await api.models.listModels();
  final models = response.data;
  if (models == null) return const [];

  return [
    for (final model in models)
      RegenerateModel(name: model.name, isDefault: model.isDefault),
  ];
});
