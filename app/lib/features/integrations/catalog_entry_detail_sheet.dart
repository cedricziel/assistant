import 'package:assistant_api/assistant_api.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../shared/platform/widgets.dart';
import '../../shared/theme/assistant_spacing.dart';
import 'auth_kind_badge.dart';
import 'caller_roles.dart';
import 'catalog_entry_tile.dart';
import 'install_integration_sheet.dart';
import 'install_integration_view_model.dart';

/// Opens the detail sheet for [entry].
Future<void> showCatalogEntryDetail(
  BuildContext context,
  CatalogEntrySummary entry,
) {
  return showModalBottomSheet<void>(
    context: context,
    isScrollControlled: true,
    useSafeArea: true,
    showDragHandle: true,
    constraints: const BoxConstraints(maxWidth: 640),
    builder: (_) => CatalogEntryDetailSheet(entry: entry),
  );
}

/// Description, domain, connect URL and auth requirements of one entry.
class CatalogEntryDetailSheet extends ConsumerWidget {
  const CatalogEntryDetailSheet({super.key, required this.entry});

  final CatalogEntrySummary entry;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);
    final muted = TextStyle(color: theme.colorScheme.onSurfaceVariant);

    return SingleChildScrollView(
      padding: const EdgeInsets.fromLTRB(
        AssistantSpacing.xl,
        0,
        AssistantSpacing.xl,
        AssistantSpacing.xl,
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        mainAxisSize: MainAxisSize.min,
        children: [
          Row(
            children: [
              CatalogEntryIcon(name: entry.name, iconUrl: entry.iconUrl),
              const SizedBox(width: AssistantSpacing.md),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(entry.name, style: theme.textTheme.titleLarge),
                    Text(entry.domain, style: muted),
                  ],
                ),
              ),
            ],
          ),
          if (entry.description.isNotEmpty) ...[
            const SizedBox(height: AssistantSpacing.lg),
            Text(entry.description),
          ],
          if (entry.categories.isNotEmpty) ...[
            const SizedBox(height: AssistantSpacing.md),
            Wrap(
              spacing: AssistantSpacing.sm,
              runSpacing: AssistantSpacing.xs,
              children: [
                for (final category in entry.categories)
                  Chip(label: Text(category)),
              ],
            ),
          ],
          const SizedBox(height: AssistantSpacing.lg),
          Text('Connect URL', style: theme.textTheme.labelLarge),
          const SizedBox(height: AssistantSpacing.xs),
          SelectableText(entry.connectUrl),
          const SizedBox(height: AssistantSpacing.lg),
          Row(
            children: [
              Text('Authentication', style: theme.textTheme.labelLarge),
              const SizedBox(width: AssistantSpacing.sm),
              AuthKindBadge(kind: entry.auth.kind),
            ],
          ),
          const SizedBox(height: AssistantSpacing.xs),
          Text(authRequirement(entry.auth)),
          const SizedBox(height: AssistantSpacing.xl),
          Align(
            alignment: Alignment.centerRight,
            child: _InstallButton(entry: entry),
          ),
        ],
      ),
    );
  }
}

/// Plain-language description of what connecting to an entry requires.
String authRequirement(CatalogAuthResponse auth) {
  final header = _headerHint(auth);
  return switch (auth.kind) {
    CatalogAuthKindResponse.none => 'No credentials needed.',
    CatalogAuthKindResponse.header => 'Needs an API key$header.',
    CatalogAuthKindResponse.oauth => 'Sign in with OAuth when you connect.',
    CatalogAuthKindResponse.mixed =>
      'Works with an API key$header or an OAuth sign-in.',
    _ => 'The catalog does not say how this server authenticates.',
  };
}

String _headerHint(CatalogAuthResponse auth) {
  final name = auth.headerName;
  if (name == null) return '';
  final value = '${auth.valuePrefix ?? ''}{${auth.placeholder ?? 'value'}}';
  return ', sent in the $name header as $value';
}

/// Opens the install sheet when the caller may install [entry] anywhere, and
/// says why not otherwise.
class _InstallButton extends ConsumerWidget {
  const _InstallButton({required this.entry});

  final CatalogEntrySummary entry;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final roles = ref.watch(callerRolesProvider);
    final (message, args) = switch (roles) {
      AsyncData(:final value) => switch (installArgsForEntry(entry, value)) {
        null => (
          'Your role or your organization\'s policy does not allow installing this.',
          null,
        ),
        final args => (null, args),
      },
      AsyncError() => ('Could not check what you may install.', null),
      _ => ('Checking what you may install…', null),
    };

    final button = AdaptiveButton.filled(
      onPressed: args == null ? null : () => _open(context, args),
      child: const Text('Install'),
    );
    return message == null ? button : Tooltip(message: message, child: button);
  }

  void _open(BuildContext context, InstallArgs args) {
    final navigator = Navigator.of(context)..pop();
    showInstallIntegrationSheet(navigator.context, args);
  }
}
