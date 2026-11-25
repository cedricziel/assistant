import 'package:assistant_api/assistant_api.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../shared/platform/widgets.dart';
import '../../shared/theme/assistant_spacing.dart';
import 'catalog_entry_tile.dart';
import 'install_integration_view_model.dart';
import 'integration_list_tile.dart';
import 'integration_list_view_model.dart';
import 'integration_summary_of.dart';

/// Opens the install sheet for [args].
Future<void> showInstallIntegrationSheet(
  BuildContext context,
  InstallArgs args,
) {
  return showModalBottomSheet<void>(
    context: context,
    isScrollControlled: true,
    useSafeArea: true,
    showDragHandle: true,
    constraints: const BoxConstraints(maxWidth: 640),
    builder: (_) => InstallIntegrationSheet(args: args),
  );
}

/// Form that installs a catalog entry, or a custom URL, as an integration.
///
/// Credentials are not entered here: a new integration that needs one shows
/// as "Needs connection" until credential setup ships.
class InstallIntegrationSheet extends ConsumerStatefulWidget {
  const InstallIntegrationSheet({super.key, required this.args});

  final InstallArgs args;

  @override
  ConsumerState<InstallIntegrationSheet> createState() =>
      _InstallIntegrationSheetState();
}

class _InstallIntegrationSheetState
    extends ConsumerState<InstallIntegrationSheet> {
  late final TextEditingController _name;
  late final TextEditingController _slug;
  final _url = TextEditingController();
  final _headerName = TextEditingController();
  final _headerPrefix = TextEditingController();

  @override
  void initState() {
    super.initState();
    final initial = ref.read(installIntegrationViewModelProvider(widget.args));
    _name = TextEditingController(text: initial.name);
    _slug = TextEditingController(text: initial.slug);
  }

  @override
  void dispose() {
    for (final controller in [_name, _slug, _url, _headerName, _headerPrefix]) {
      controller.dispose();
    }
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final provider = installIntegrationViewModelProvider(widget.args);
    final form = ref.watch(provider);
    final vm = ref.read(provider.notifier);
    ref.listen(provider.select((s) => s.created), (_, created) {
      if (created != null) _refreshList(ref, created);
    });

    final created = form.created;
    return SingleChildScrollView(
      padding: EdgeInsets.fromLTRB(
        AssistantSpacing.xl,
        0,
        AssistantSpacing.xl,
        AssistantSpacing.xl + MediaQuery.viewInsetsOf(context).bottom,
      ),
      child: created == null
          ? _form(context, form, vm)
          : _installed(context, created),
    );
  }

  Widget _installed(BuildContext context, IntegrationDetail created) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      mainAxisSize: MainAxisSize.min,
      children: [
        Text('Installed', style: Theme.of(context).textTheme.titleLarge),
        const SizedBox(height: AssistantSpacing.md),
        IntegrationListTile(integration: integrationSummaryOf(created)),
        const SizedBox(height: AssistantSpacing.lg),
        Align(
          alignment: Alignment.centerRight,
          child: AdaptiveButton.filled(
            onPressed: () => Navigator.of(context).pop(),
            child: const Text('Done'),
          ),
        ),
      ],
    );
  }

  Widget _form(
    BuildContext context,
    InstallState form,
    InstallIntegrationViewModel vm,
  ) {
    final theme = Theme.of(context);
    final entry = widget.args.entry;
    final muted = TextStyle(color: theme.colorScheme.onSurfaceVariant);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      mainAxisSize: MainAxisSize.min,
      children: [
        Row(
          children: [
            CatalogEntryIcon(
              name: entry?.name ?? 'Custom',
              iconUrl: entry?.iconUrl,
            ),
            const SizedBox(width: AssistantSpacing.md),
            Expanded(
              child: Text(
                entry == null
                    ? 'Add custom integration'
                    : 'Install ${entry.name}',
                style: theme.textTheme.titleLarge,
              ),
            ),
          ],
        ),
        if (entry == null) ...[
          const SizedBox(height: AssistantSpacing.lg),
          TextFormField(
            controller: _url,
            keyboardType: TextInputType.url,
            decoration: InputDecoration(
              labelText: 'Server URL',
              hintText: 'https://mcp.example.com/mcp',
              errorText: _url.text.isEmpty ? null : form.urlError,
            ),
            onChanged: vm.setConnectUrl,
          ),
        ],
        _label(context, 'Where should it be available?'),
        _chips<IntegrationLayer>(
          values: widget.args.layers,
          selected: form.layer,
          label: _layerLabel,
          onSelected: vm.setLayer,
        ),
        if (form.layer == IntegrationLayer.space) ...[
          _label(context, 'Space'),
          _chips<String>(
            values: [for (final space in widget.args.spaces) space.id],
            selected: form.spaceId,
            label: (id) =>
                widget.args.spaces.firstWhere((space) => space.id == id).name,
            onSelected: vm.setSpace,
          ),
        ],
        const SizedBox(height: AssistantSpacing.lg),
        TextFormField(
          controller: _name,
          decoration: InputDecoration(
            labelText: 'Name',
            errorText: form.nameError,
          ),
          onChanged: vm.setName,
        ),
        const SizedBox(height: AssistantSpacing.md),
        TextFormField(
          controller: _slug,
          decoration: InputDecoration(
            labelText: 'Slug',
            helperText:
                'Used in tool names. Lower-case letters, digits, hyphens.',
            errorText: form.slugError,
          ),
          onChanged: vm.setSlug,
        ),
        if (form.needsAuthChoice) ...[
          _label(context, 'How does it sign in?'),
          _chips<IntegrationAuthKind>(
            values: const [
              IntegrationAuthKind.none,
              IntegrationAuthKind.header,
              IntegrationAuthKind.oauth,
            ],
            selected: form.authKind,
            label: _authLabel,
            onSelected: vm.setAuthKind,
          ),
        ],
        if (form.needsHeaderName) ...[
          const SizedBox(height: AssistantSpacing.md),
          TextFormField(
            controller: _headerName,
            decoration: const InputDecoration(
              labelText: 'Header name',
              hintText: 'Authorization',
            ),
            onChanged: vm.setHeaderName,
          ),
          const SizedBox(height: AssistantSpacing.md),
          TextFormField(
            controller: _headerPrefix,
            decoration: const InputDecoration(
              labelText: 'Value prefix (optional)',
              hintText: 'Bearer ',
            ),
            onChanged: vm.setHeaderPrefix,
          ),
        ],
        if (form.showsCredentialMode) ...[
          _label(context, 'Who connects?'),
          _chips<CredentialMode>(
            values: const [CredentialMode.perUser, CredentialMode.shared],
            selected: form.credentialMode,
            label: (mode) => mode == CredentialMode.perUser
                ? 'Each person connects their own'
                : 'One shared credential',
            onSelected: vm.setCredentialMode,
          ),
          if (form.credentialMode == CredentialMode.shared &&
              (form.effectiveAuthKind == IntegrationAuthKind.oauth ||
                  form.effectiveAuthKind == IntegrationAuthKind.mixed)) ...[
            const SizedBox(height: AssistantSpacing.xs),
            Text(
              'A shared OAuth sign-in acts as one person for everyone who '
              'uses this integration.',
              style: TextStyle(color: theme.colorScheme.error),
            ),
          ],
        ],
        const SizedBox(height: AssistantSpacing.md),
        AdaptiveSwitchTile(
          title: const Text('Trust its tool calls'),
          subtitle: Text(
            'Skip the confirmation step. By default you confirm each call.',
            style: muted,
          ),
          value: form.trust == IntegrationTrust.trust,
          onChanged: (on) => vm.setTrust(
            on ? IntegrationTrust.trust : IntegrationTrust.confirm,
          ),
        ),
        if (form.error != null) ...[
          const SizedBox(height: AssistantSpacing.sm),
          Text(form.error!, style: TextStyle(color: theme.colorScheme.error)),
        ],
        const SizedBox(height: AssistantSpacing.lg),
        Align(
          alignment: Alignment.centerRight,
          child: AdaptiveButton.filled(
            onPressed: form.canSubmit ? vm.submit : null,
            child: form.submitting
                ? const SizedBox.square(
                    dimension: 18,
                    child: CircularProgressIndicator(strokeWidth: 2),
                  )
                : const Text('Install'),
          ),
        ),
      ],
    );
  }

  Widget _label(BuildContext context, String text) => Padding(
    padding: const EdgeInsets.only(
      top: AssistantSpacing.lg,
      bottom: AssistantSpacing.sm,
    ),
    child: Text(text, style: Theme.of(context).textTheme.labelLarge),
  );

  Widget _chips<T>({
    required List<T> values,
    required T? selected,
    required String Function(T) label,
    required ValueChanged<T> onSelected,
  }) {
    return Wrap(
      spacing: AssistantSpacing.sm,
      runSpacing: AssistantSpacing.xs,
      children: [
        for (final value in values)
          ChoiceChip(
            label: Text(label(value)),
            selected: value == selected,
            onSelected: (_) => onSelected(value),
          ),
      ],
    );
  }
}

String _layerLabel(IntegrationLayer layer) => switch (layer) {
  IntegrationLayer.org => 'Whole organization',
  IntegrationLayer.space => 'One space',
  _ => 'Just me',
};

String _authLabel(IntegrationAuthKind kind) => switch (kind) {
  IntegrationAuthKind.none => 'No auth',
  IntegrationAuthKind.header => 'API key',
  _ => 'OAuth',
};

/// Reloads the list the new integration appears in.
void _refreshList(WidgetRef ref, IntegrationDetail created) {
  final scope = switch (created.layer) {
    IntegrationLayer.org => const IntegrationListScope.org(),
    IntegrationLayer.space => IntegrationListScope.space(created.spaceId!),
    _ => const IntegrationListScope.connections(),
  };
  ref.invalidate(integrationListViewModelProvider(scope));
}
