import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../../shared/platform/widgets.dart';
import '../models/context_model.dart';
import '../providers/context_providers.dart';
import 'edit_context_screen.dart';

/// Full-screen list of saved [AssistantContext] objects.
///
/// Shown on launch when no context is active (router redirect) and accessible
/// from the nav rail at any time.
class ContextSwitcherScreen extends ConsumerWidget {
  const ContextSwitcherScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final contextsAsync = ref.watch(contextsProvider);
    final activeAsync = ref.watch(activeContextProvider);
    final activeContext = activeAsync.value;

    final fab = FloatingActionButton(
      tooltip: 'Add context',
      onPressed: () => _openCreate(context, ref),
      child: const Icon(Icons.add),
    );

    return AdaptiveScaffold(
      body: CustomScrollView(
        slivers: [
          const AdaptiveSliverNavBar(title: 'Contexts'),
          contextsAsync.when(
            loading: () => const SliverFillRemaining(
              child: Center(child: CircularProgressIndicator.adaptive()),
            ),
            error: (e, _) =>
                SliverFillRemaining(child: Center(child: Text('Error: $e'))),
            data: (contexts) {
              if (contexts.isEmpty) {
                return const SliverFillRemaining(child: _EmptyState());
              }
              return SliverList(
                delegate: SliverChildBuilderDelegate((ctx, i) {
                  final item = contexts[i];
                  final isActive = activeContext?.id == item.id;
                  return _ContextTile(
                    context: item,
                    isActive: isActive,
                    onTap: () => _activateAndNavigate(ctx, ref, item),
                    onEdit: () => _openEdit(ctx, item),
                    onDelete: () => _confirmDelete(ctx, ref, item),
                  );
                }, childCount: contexts.length),
              );
            },
          ),
          const SliverToBoxAdapter(child: SizedBox(height: 80)),
        ],
      ),
      floatingActionButton: fab,
    );
  }

  Future<void> _activateAndNavigate(
    BuildContext context,
    WidgetRef ref,
    AssistantContext ctx,
  ) async {
    await ref.read(activeContextProvider.notifier).activate(ctx);
    if (context.mounted) {
      context.go('/chat');
    }
  }

  Future<void> _openCreate(BuildContext context, WidgetRef ref) async {
    await showDialog<void>(
      context: context,
      builder: (ctx) => const _CreateContextDialog(),
    );
  }

  Future<void> _openEdit(BuildContext context, AssistantContext ctx) async {
    await Navigator.of(context).push<void>(
      MaterialPageRoute(
        builder: (_) => EditContextScreen(existingContext: ctx),
      ),
    );
  }

  Future<void> _confirmDelete(
    BuildContext context,
    WidgetRef ref,
    AssistantContext ctx,
  ) async {
    final confirmed = await showAdaptiveConfirmDialog(
      context: context,
      title: 'Delete context?',
      content: 'Remove "${ctx.name}"? This cannot be undone.',
      confirmLabel: 'Delete',
      isDestructive: true,
    );
    if (confirmed) {
      await ref.read(contextsProvider.notifier).deleteContext(ctx.id);
      // If active context was deleted, deactivate it.
      final activeId = ref.read(activeContextProvider).value?.id;
      if (activeId == ctx.id) {
        await ref.read(activeContextProvider.notifier).deactivate();
      }
    }
  }
}

// -- Empty state -------------------------------------------------------------

class _EmptyState extends StatelessWidget {
  const _EmptyState();

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(
            Icons.swap_horiz_outlined,
            size: 64,
            color: Theme.of(context).colorScheme.outline,
          ),
          const SizedBox(height: 16),
          Text(
            'No contexts yet. Tap + to add one.',
            style: Theme.of(context).textTheme.bodyLarge?.copyWith(
              color: Theme.of(context).colorScheme.outline,
            ),
          ),
        ],
      ),
    );
  }
}

// -- Context tile ------------------------------------------------------------

class _ContextTile extends StatelessWidget {
  const _ContextTile({
    required this.context,
    required this.isActive,
    required this.onTap,
    required this.onEdit,
    required this.onDelete,
  });

  final AssistantContext context;
  final bool isActive;
  final VoidCallback onTap;
  final VoidCallback onEdit;
  final VoidCallback onDelete;

  @override
  Widget build(BuildContext buildContext) {
    return Semantics(
      label:
          '${context.name}, ${context.serverUrl}${isActive ? ', active' : ''}',
      button: true,
      child: AdaptiveListTile(
        leading: isActive
            ? Icon(
                Icons.check_circle,
                color: Theme.of(buildContext).colorScheme.primary,
                semanticLabel: 'Active',
              )
            : const Icon(Icons.radio_button_unchecked),
        title: Text(context.name),
        subtitle: Text(context.serverUrl, overflow: TextOverflow.ellipsis),
        trailing: SizedBox(
          width: 96,
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              IconButton(
                icon: const Icon(Icons.edit_outlined),
                tooltip: 'Edit',
                onPressed: onEdit,
              ),
              IconButton(
                icon: const Icon(Icons.delete_outline),
                tooltip: 'Delete',
                onPressed: onDelete,
              ),
            ],
          ),
        ),
        onTap: onTap,
        onLongPress: onDelete,
      ),
    );
  }
}

// -- Create context dialog ---------------------------------------------------

class _CreateContextDialog extends ConsumerStatefulWidget {
  const _CreateContextDialog();

  @override
  ConsumerState<_CreateContextDialog> createState() =>
      _CreateContextDialogState();
}

class _CreateContextDialogState extends ConsumerState<_CreateContextDialog> {
  final _formKey = GlobalKey<FormState>();
  final _nameCtrl = TextEditingController();
  final _urlCtrl = TextEditingController();
  final _tokenCtrl = TextEditingController();
  bool _tokenVisible = false;
  bool _saving = false;

  @override
  void dispose() {
    _nameCtrl.dispose();
    _urlCtrl.dispose();
    _tokenCtrl.dispose();
    super.dispose();
  }

  Future<void> _save() async {
    if (!_formKey.currentState!.validate()) return;

    setState(() => _saving = true);
    try {
      final ctx = AssistantContext.create(
        name: _nameCtrl.text.trim(),
        serverUrl: _urlCtrl.text.trim().replaceAll(RegExp(r'/$'), ''),
        authToken: _tokenCtrl.text.trim().isEmpty
            ? null
            : _tokenCtrl.text.trim(),
      );
      await ref.read(contextsProvider.notifier).saveContext(ctx);
      if (mounted) Navigator.of(context).pop();
    } finally {
      if (mounted) setState(() => _saving = false);
    }
  }

  String? _validateName(String? value) {
    if (value == null || value.trim().isEmpty) return 'Name is required';
    final existing = ref.read(contextsProvider).value ?? [];
    final duplicate = existing.any(
      (c) => c.name.toLowerCase() == value.trim().toLowerCase(),
    );
    if (duplicate) return 'A context with this name already exists';
    return null;
  }

  String? _validateUrl(String? value) {
    if (value == null || value.trim().isEmpty) return 'URL is required';
    final uri = Uri.tryParse(value.trim());
    if (uri == null || !uri.hasScheme || !uri.hasAuthority) {
      return 'Enter a valid URL (e.g. http://localhost:8080)';
    }
    return null;
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('New Context'),
      content: SingleChildScrollView(
        child: Form(
          key: _formKey,
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              TextFormField(
                controller: _nameCtrl,
                decoration: const InputDecoration(labelText: 'Name'),
                validator: _validateName,
                textInputAction: TextInputAction.next,
              ),
              const SizedBox(height: 8),
              TextFormField(
                controller: _urlCtrl,
                decoration: const InputDecoration(
                  labelText: 'URL',
                  hintText: 'https://assistant.example.com',
                ),
                validator: _validateUrl,
                keyboardType: TextInputType.url,
                textInputAction: TextInputAction.next,
              ),
              const SizedBox(height: 8),
              TextFormField(
                controller: _tokenCtrl,
                decoration: InputDecoration(
                  labelText: 'Token (optional)',
                  suffixIcon: IconButton(
                    icon: Icon(
                      _tokenVisible ? Icons.visibility_off : Icons.visibility,
                    ),
                    onPressed: () =>
                        setState(() => _tokenVisible = !_tokenVisible),
                  ),
                ),
                obscureText: !_tokenVisible,
                textInputAction: TextInputAction.done,
                onFieldSubmitted: (_) => _save(),
              ),
            ],
          ),
        ),
      ),
      actions: [
        AdaptiveButton.plain(
          onPressed: _saving ? null : () => Navigator.of(context).pop(),
          child: const Text('Cancel'),
        ),
        AdaptiveButton.filled(
          onPressed: _saving ? null : _save,
          child: _saving
              ? const SizedBox(
                  width: 16,
                  height: 16,
                  child: CircularProgressIndicator.adaptive(strokeWidth: 2),
                )
              : const Text('Save'),
        ),
      ],
    );
  }
}
