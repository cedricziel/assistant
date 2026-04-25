import 'package:assistant_api/assistant_api.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'api_keys_provider.dart';

/// Screen for managing the current user's API keys.
///
/// Displays existing keys with prefix + creation date, allows creating new
/// keys (with copy-to-clipboard on creation), and revoking existing ones.
class ApiKeysScreen extends ConsumerWidget {
  const ApiKeysScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final keysAsync = ref.watch(apiKeysProvider);

    return Scaffold(
      appBar: AppBar(title: const Text('API Keys')),
      floatingActionButton: FloatingActionButton(
        onPressed: () => _showCreateDialog(context, ref),
        tooltip: 'Create API key',
        child: const Icon(Icons.add),
      ),
      body: keysAsync.when(
        loading: () =>
            const Center(child: CircularProgressIndicator.adaptive()),
        error: (err, _) => Center(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Text('Failed to load API keys: $err'),
              const SizedBox(height: 12),
              FilledButton.tonal(
                onPressed: () => ref.invalidate(apiKeysProvider),
                child: const Text('Retry'),
              ),
            ],
          ),
        ),
        data: (keys) {
          if (keys.isEmpty) {
            return const Center(
              child: Padding(
                padding: EdgeInsets.all(32),
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Icon(Icons.key_off, size: 48, color: Colors.grey),
                    SizedBox(height: 16),
                    Text(
                      'No API keys',
                      style: TextStyle(
                        fontSize: 18,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                    SizedBox(height: 8),
                    Text(
                      'Create an API key to authenticate programmatic access.',
                      textAlign: TextAlign.center,
                    ),
                  ],
                ),
              ),
            );
          }

          return ListView.separated(
            padding: const EdgeInsets.all(16),
            itemCount: keys.length,
            separatorBuilder: (_, _) => const SizedBox(height: 8),
            itemBuilder: (context, index) =>
                _ApiKeyTile(key: ValueKey(keys[index].id), apiKey: keys[index]),
          );
        },
      ),
    );
  }

  void _showCreateDialog(BuildContext context, WidgetRef ref) {
    showDialog<void>(
      context: context,
      builder: (dialogContext) => _CreateApiKeyDialog(ref: ref),
    );
  }
}

class _ApiKeyTile extends ConsumerWidget {
  const _ApiKeyTile({super.key, required this.apiKey});

  final ApiKeySummary apiKey;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final colorScheme = Theme.of(context).colorScheme;

    return Card(
      child: ListTile(
        leading: Icon(Icons.key, color: colorScheme.primary),
        title: Text(apiKey.name),
        subtitle: Text(
          '${apiKey.keyPrefix}... \u00B7 Created ${apiKey.createdAt}',
          style: TextStyle(color: colorScheme.onSurfaceVariant, fontSize: 12),
        ),
        trailing: IconButton(
          icon: Icon(Icons.delete_outline, color: colorScheme.error),
          tooltip: 'Revoke',
          onPressed: () => _confirmRevoke(context, ref),
        ),
      ),
    );
  }

  void _confirmRevoke(BuildContext context, WidgetRef ref) {
    showDialog<void>(
      context: context,
      builder: (dialogContext) => AlertDialog(
        title: const Text('Revoke API key?'),
        content: Text(
          'This will permanently revoke "${apiKey.name}". '
          'Any applications using this key will lose access.',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(dialogContext).pop(),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () {
              Navigator.of(dialogContext).pop();
              ref.read(apiKeysProvider.notifier).revokeKey(apiKey.id);
            },
            style: FilledButton.styleFrom(
              backgroundColor: Theme.of(context).colorScheme.error,
            ),
            child: const Text('Revoke'),
          ),
        ],
      ),
    );
  }
}

class _CreateApiKeyDialog extends StatefulWidget {
  const _CreateApiKeyDialog({required this.ref});

  final WidgetRef ref;

  @override
  State<_CreateApiKeyDialog> createState() => _CreateApiKeyDialogState();
}

class _CreateApiKeyDialogState extends State<_CreateApiKeyDialog> {
  final _nameController = TextEditingController();
  bool _isCreating = false;
  CreateApiKeyResponse? _result;

  @override
  void dispose() {
    _nameController.dispose();
    super.dispose();
  }

  Future<void> _create() async {
    if (_nameController.text.trim().isEmpty) return;

    setState(() => _isCreating = true);
    try {
      final response = await widget.ref
          .read(apiKeysProvider.notifier)
          .createKey(name: _nameController.text.trim());
      if (mounted && response != null) {
        setState(() => _result = response);
      }
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(SnackBar(content: Text('Failed to create key: $e')));
        Navigator.of(context).pop();
      }
    } finally {
      if (mounted) setState(() => _isCreating = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    if (_result != null) {
      return AlertDialog(
        title: const Text('API key created'),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            const Text(
              'Copy your API key now. You will not be able to see it again.',
              style: TextStyle(fontWeight: FontWeight.w500),
            ),
            const SizedBox(height: 16),
            SelectableText(
              _result!.plaintext,
              style: const TextStyle(fontFamily: 'monospace', fontSize: 13),
            ),
            const SizedBox(height: 12),
            OutlinedButton.icon(
              onPressed: () {
                Clipboard.setData(ClipboardData(text: _result!.plaintext));
                ScaffoldMessenger.of(context).showSnackBar(
                  const SnackBar(content: Text('Copied to clipboard')),
                );
              },
              icon: const Icon(Icons.copy, size: 18),
              label: const Text('Copy to clipboard'),
            ),
          ],
        ),
        actions: [
          FilledButton(
            onPressed: () => Navigator.of(context).pop(),
            child: const Text('Done'),
          ),
        ],
      );
    }

    return AlertDialog(
      title: const Text('Create API key'),
      content: TextField(
        controller: _nameController,
        autofocus: true,
        decoration: const InputDecoration(
          labelText: 'Key name',
          hintText: 'e.g. CI/CD Pipeline',
        ),
        onSubmitted: (_) => _create(),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('Cancel'),
        ),
        FilledButton(
          onPressed: _isCreating ? null : _create,
          child: _isCreating
              ? const SizedBox(
                  width: 16,
                  height: 16,
                  child: CircularProgressIndicator(strokeWidth: 2),
                )
              : const Text('Create'),
        ),
      ],
    );
  }
}
