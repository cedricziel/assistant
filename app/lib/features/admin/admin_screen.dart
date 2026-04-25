import 'package:assistant_api/assistant_api.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'admin_providers.dart';

/// Admin screen with tabs for user, space, and catalog management.
///
/// Intended for org-admin and space-admin users. The server enforces role
/// checks — the UI surfaces the management actions and shows errors if the
/// user lacks permission.
class AdminScreen extends ConsumerStatefulWidget {
  const AdminScreen({super.key});

  @override
  ConsumerState<AdminScreen> createState() => _AdminScreenState();
}

class _AdminScreenState extends ConsumerState<AdminScreen>
    with SingleTickerProviderStateMixin {
  late final TabController _tabController;

  @override
  void initState() {
    super.initState();
    _tabController = TabController(length: 3, vsync: this);
    _tabController.addListener(() {
      if (mounted) setState(() {});
    });
  }

  @override
  void dispose() {
    _tabController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Admin'),
        bottom: TabBar(
          controller: _tabController,
          tabs: const [
            Tab(text: 'Users'),
            Tab(text: 'Spaces'),
            Tab(text: 'Catalog'),
          ],
        ),
      ),
      floatingActionButton: _buildFab(context),
      body: TabBarView(
        controller: _tabController,
        children: const [_UsersTab(), _SpacesTab(), _CatalogTab()],
      ),
    );
  }

  Widget? _buildFab(BuildContext context) {
    switch (_tabController.index) {
      case 0:
        return FloatingActionButton(
          onPressed: () => _showInviteDialog(context),
          tooltip: 'Invite user',
          child: const Icon(Icons.person_add),
        );
      case 1:
        return FloatingActionButton(
          onPressed: () => _showCreateSpaceDialog(context),
          tooltip: 'Create space',
          child: const Icon(Icons.add),
        );
      case 2:
        return FloatingActionButton(
          onPressed: () => _showPublishDialog(context),
          tooltip: 'Publish to catalog',
          child: const Icon(Icons.publish),
        );
      default:
        return null;
    }
  }

  void _showInviteDialog(BuildContext context) {
    showDialog<void>(
      context: context,
      builder: (dialogContext) => _InviteUserDialog(ref: ref),
    );
  }

  void _showCreateSpaceDialog(BuildContext context) {
    showDialog<void>(
      context: context,
      builder: (dialogContext) => _CreateSpaceDialog(ref: ref),
    );
  }

  void _showPublishDialog(BuildContext context) {
    showDialog<void>(
      context: context,
      builder: (dialogContext) => _PublishCatalogDialog(ref: ref),
    );
  }
}

// ---------------------------------------------------------------------------
// Users Tab

class _UsersTab extends ConsumerWidget {
  const _UsersTab();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final usersAsync = ref.watch(adminUsersProvider);

    return usersAsync.when(
      loading: () => const Center(child: CircularProgressIndicator.adaptive()),
      error: (err, _) => _ErrorView(
        message: 'Failed to load users: $err',
        onRetry: () => ref.invalidate(adminUsersProvider),
      ),
      data: (users) {
        if (users.isEmpty) {
          return const _EmptyView(
            icon: Icons.people_outline,
            title: 'No users',
            subtitle: 'Invite users to your organization to get started.',
          );
        }

        return ListView.separated(
          padding: const EdgeInsets.all(16),
          itemCount: users.length,
          separatorBuilder: (_, _) => const SizedBox(height: 8),
          itemBuilder: (context, index) {
            final user = users[index];
            return _UserTile(user: user);
          },
        );
      },
    );
  }
}

class _UserTile extends ConsumerWidget {
  const _UserTile({required this.user});
  final UserSummary user;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final colorScheme = Theme.of(context).colorScheme;

    return Card(
      child: ListTile(
        leading: CircleAvatar(
          backgroundColor: colorScheme.primaryContainer,
          child: Text(
            user.name.isNotEmpty ? user.name[0].toUpperCase() : '?',
            style: TextStyle(color: colorScheme.onPrimaryContainer),
          ),
        ),
        title: Text(user.name),
        subtitle: Text(user.email),
        trailing: IconButton(
          icon: Icon(Icons.delete_outline, color: colorScheme.error),
          tooltip: 'Remove user',
          onPressed: () => _confirmRemove(context, ref),
        ),
      ),
    );
  }

  void _confirmRemove(BuildContext context, WidgetRef ref) {
    showDialog<void>(
      context: context,
      builder: (dialogContext) => AlertDialog(
        title: const Text('Remove user?'),
        content: Text(
          'This will remove "${user.name}" from the organization. '
          'They will lose access to all spaces.',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(dialogContext).pop(),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () {
              Navigator.of(dialogContext).pop();
              ref.read(adminUsersProvider.notifier).removeUser(user.id);
            },
            style: FilledButton.styleFrom(
              backgroundColor: Theme.of(context).colorScheme.error,
            ),
            child: const Text('Remove'),
          ),
        ],
      ),
    );
  }
}

// ---------------------------------------------------------------------------
// Spaces Tab

class _SpacesTab extends ConsumerWidget {
  const _SpacesTab();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final spacesAsync = ref.watch(adminSpacesProvider);
    final membersAsync = ref.watch(adminMembersProvider);

    return SingleChildScrollView(
      padding: const EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            'Spaces',
            style: Theme.of(context).textTheme.titleMedium?.copyWith(
              color: Theme.of(context).colorScheme.primary,
            ),
          ),
          const SizedBox(height: 8),
          spacesAsync.when(
            loading: () =>
                const Center(child: CircularProgressIndicator.adaptive()),
            error: (err, _) => _ErrorView(
              message: 'Failed to load spaces: $err',
              onRetry: () => ref.invalidate(adminSpacesProvider),
            ),
            data: (spaces) {
              if (spaces.isEmpty) {
                return const _EmptyView(
                  icon: Icons.workspaces_outline,
                  title: 'No spaces',
                  subtitle: 'Create a space to organize your team.',
                );
              }

              return Column(
                children: [
                  for (final space in spaces) _SpaceTile(space: space),
                ],
              );
            },
          ),
          const SizedBox(height: 24),
          Text(
            'Members',
            style: Theme.of(context).textTheme.titleMedium?.copyWith(
              color: Theme.of(context).colorScheme.primary,
            ),
          ),
          const SizedBox(height: 8),
          membersAsync.when(
            loading: () =>
                const Center(child: CircularProgressIndicator.adaptive()),
            error: (err, _) => _ErrorView(
              message: 'Failed to load members: $err',
              onRetry: () => ref.invalidate(adminMembersProvider),
            ),
            data: (members) {
              if (members.isEmpty) {
                return const _EmptyView(
                  icon: Icons.group_outlined,
                  title: 'No members',
                  subtitle: 'Add members to this space.',
                );
              }

              return Column(
                children: [for (final m in members) _MemberTile(member: m)],
              );
            },
          ),
        ],
      ),
    );
  }
}

class _SpaceTile extends ConsumerWidget {
  const _SpaceTile({required this.space});
  final SpaceSummary space;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final colorScheme = Theme.of(context).colorScheme;

    return Card(
      child: ListTile(
        leading: Icon(Icons.workspaces, color: colorScheme.primary),
        title: Text(space.name),
        subtitle: Text(space.slug),
        trailing: IconButton(
          icon: Icon(Icons.delete_outline, color: colorScheme.error),
          tooltip: 'Delete space',
          onPressed: () => _confirmDelete(context, ref),
        ),
      ),
    );
  }

  void _confirmDelete(BuildContext context, WidgetRef ref) {
    showDialog<void>(
      context: context,
      builder: (dialogContext) => AlertDialog(
        title: const Text('Delete space?'),
        content: Text(
          'This will permanently delete "${space.name}" and all its data.',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(dialogContext).pop(),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () {
              Navigator.of(dialogContext).pop();
              ref.read(adminSpacesProvider.notifier).deleteSpace(space.id);
            },
            style: FilledButton.styleFrom(
              backgroundColor: Theme.of(context).colorScheme.error,
            ),
            child: const Text('Delete'),
          ),
        ],
      ),
    );
  }
}

class _MemberTile extends ConsumerWidget {
  const _MemberTile({required this.member});
  final MemberEntry member;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final colorScheme = Theme.of(context).colorScheme;

    return Card(
      child: ListTile(
        leading: Icon(Icons.person, color: colorScheme.primary),
        title: Text(member.userId),
        subtitle: Text(member.role),
        trailing: IconButton(
          icon: Icon(Icons.remove_circle_outline, color: colorScheme.error),
          tooltip: 'Remove from space',
          onPressed: () {
            ref.read(adminMembersProvider.notifier).removeMember(member.userId);
          },
        ),
      ),
    );
  }
}

// ---------------------------------------------------------------------------
// Catalog Tab

class _CatalogTab extends ConsumerWidget {
  const _CatalogTab();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final catalogAsync = ref.watch(adminCatalogProvider);

    return catalogAsync.when(
      loading: () => const Center(child: CircularProgressIndicator.adaptive()),
      error: (err, _) => _ErrorView(
        message: 'Failed to load catalog: $err',
        onRetry: () => ref.invalidate(adminCatalogProvider),
      ),
      data: (items) {
        if (items.isEmpty) {
          return const _EmptyView(
            icon: Icons.store_outlined,
            title: 'No catalog items',
            subtitle:
                'Publish skills, templates, or interfaces to the catalog.',
          );
        }

        return ListView.separated(
          padding: const EdgeInsets.all(16),
          itemCount: items.length,
          separatorBuilder: (_, _) => const SizedBox(height: 8),
          itemBuilder: (context, index) {
            final item = items[index];
            return _CatalogTile(item: item);
          },
        );
      },
    );
  }
}

class _CatalogTile extends ConsumerWidget {
  const _CatalogTile({required this.item});
  final CatalogItemResponse item;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final colorScheme = Theme.of(context).colorScheme;
    final typeIcon = switch (item.resourceType) {
      'skill' => Icons.extension,
      'template' => Icons.description,
      'interface' => Icons.cable,
      _ => Icons.inventory_2,
    };

    return Card(
      child: ListTile(
        leading: Icon(typeIcon, color: colorScheme.primary),
        title: Text(item.name),
        subtitle: Text(
          '${item.resourceType} \u00B7 ${item.description}',
          maxLines: 1,
          overflow: TextOverflow.ellipsis,
        ),
        trailing: IconButton(
          icon: Icon(Icons.delete_outline, color: colorScheme.error),
          tooltip: 'Remove from catalog',
          onPressed: () => _confirmRemove(context, ref),
        ),
      ),
    );
  }

  void _confirmRemove(BuildContext context, WidgetRef ref) {
    showDialog<void>(
      context: context,
      builder: (dialogContext) => AlertDialog(
        title: const Text('Remove from catalog?'),
        content: Text(
          'This will remove "${item.name}" from the catalog. '
          'Existing subscriptions will be unaffected.',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(dialogContext).pop(),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () {
              Navigator.of(dialogContext).pop();
              ref.read(adminCatalogProvider.notifier).removeItem(item.id);
            },
            style: FilledButton.styleFrom(
              backgroundColor: Theme.of(context).colorScheme.error,
            ),
            child: const Text('Remove'),
          ),
        ],
      ),
    );
  }
}

// ---------------------------------------------------------------------------
// Dialogs

class _InviteUserDialog extends StatefulWidget {
  const _InviteUserDialog({required this.ref});
  final WidgetRef ref;

  @override
  State<_InviteUserDialog> createState() => _InviteUserDialogState();
}

class _InviteUserDialogState extends State<_InviteUserDialog> {
  final _emailController = TextEditingController();
  final _nameController = TextEditingController();
  bool _isSubmitting = false;

  @override
  void dispose() {
    _emailController.dispose();
    _nameController.dispose();
    super.dispose();
  }

  Future<void> _submit() async {
    final email = _emailController.text.trim();
    final name = _nameController.text.trim();
    if (email.isEmpty || name.isEmpty) return;

    setState(() => _isSubmitting = true);
    try {
      await widget.ref
          .read(adminUsersProvider.notifier)
          .inviteUser(email: email, name: name);
      if (mounted) Navigator.of(context).pop();
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(SnackBar(content: Text('Failed to invite user: $e')));
        Navigator.of(context).pop();
      }
    } finally {
      if (mounted) setState(() => _isSubmitting = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('Invite user'),
      content: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          TextField(
            controller: _emailController,
            autofocus: true,
            decoration: const InputDecoration(
              labelText: 'Email',
              hintText: 'user@example.com',
            ),
            keyboardType: TextInputType.emailAddress,
          ),
          const SizedBox(height: 12),
          TextField(
            controller: _nameController,
            decoration: const InputDecoration(
              labelText: 'Name',
              hintText: 'Jane Doe',
            ),
            onSubmitted: (_) => _submit(),
          ),
        ],
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('Cancel'),
        ),
        FilledButton(
          onPressed: _isSubmitting ? null : _submit,
          child: _isSubmitting
              ? const SizedBox(
                  width: 16,
                  height: 16,
                  child: CircularProgressIndicator(strokeWidth: 2),
                )
              : const Text('Invite'),
        ),
      ],
    );
  }
}

class _CreateSpaceDialog extends StatefulWidget {
  const _CreateSpaceDialog({required this.ref});
  final WidgetRef ref;

  @override
  State<_CreateSpaceDialog> createState() => _CreateSpaceDialogState();
}

class _CreateSpaceDialogState extends State<_CreateSpaceDialog> {
  final _nameController = TextEditingController();
  final _slugController = TextEditingController();
  bool _isSubmitting = false;

  @override
  void dispose() {
    _nameController.dispose();
    _slugController.dispose();
    super.dispose();
  }

  Future<void> _submit() async {
    final name = _nameController.text.trim();
    final slug = _slugController.text.trim();
    if (name.isEmpty || slug.isEmpty) return;

    setState(() => _isSubmitting = true);
    try {
      await widget.ref
          .read(adminSpacesProvider.notifier)
          .createSpace(name: name, slug: slug);
      if (mounted) Navigator.of(context).pop();
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(SnackBar(content: Text('Failed to create space: $e')));
        Navigator.of(context).pop();
      }
    } finally {
      if (mounted) setState(() => _isSubmitting = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('Create space'),
      content: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          TextField(
            controller: _nameController,
            autofocus: true,
            decoration: const InputDecoration(
              labelText: 'Space name',
              hintText: 'e.g. Production',
            ),
          ),
          const SizedBox(height: 12),
          TextField(
            controller: _slugController,
            decoration: const InputDecoration(
              labelText: 'Slug',
              hintText: 'e.g. production',
            ),
            onSubmitted: (_) => _submit(),
          ),
        ],
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('Cancel'),
        ),
        FilledButton(
          onPressed: _isSubmitting ? null : _submit,
          child: _isSubmitting
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

class _PublishCatalogDialog extends StatefulWidget {
  const _PublishCatalogDialog({required this.ref});
  final WidgetRef ref;

  @override
  State<_PublishCatalogDialog> createState() => _PublishCatalogDialogState();
}

class _PublishCatalogDialogState extends State<_PublishCatalogDialog> {
  final _nameController = TextEditingController();
  final _descController = TextEditingController();
  String _resourceType = 'skill';
  bool _isSubmitting = false;

  @override
  void dispose() {
    _nameController.dispose();
    _descController.dispose();
    super.dispose();
  }

  Future<void> _submit() async {
    final name = _nameController.text.trim();
    if (name.isEmpty) return;

    setState(() => _isSubmitting = true);
    try {
      await widget.ref
          .read(adminCatalogProvider.notifier)
          .publishItem(
            name: name,
            resourceType: _resourceType,
            description: _descController.text.trim().isEmpty
                ? null
                : _descController.text.trim(),
          );
      if (mounted) Navigator.of(context).pop();
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(SnackBar(content: Text('Failed to publish: $e')));
        Navigator.of(context).pop();
      }
    } finally {
      if (mounted) setState(() => _isSubmitting = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('Publish to catalog'),
      content: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          TextField(
            controller: _nameController,
            autofocus: true,
            decoration: const InputDecoration(
              labelText: 'Name',
              hintText: 'e.g. Customer Support Skill',
            ),
          ),
          const SizedBox(height: 12),
          TextField(
            controller: _descController,
            decoration: const InputDecoration(
              labelText: 'Description (optional)',
            ),
            maxLines: 2,
          ),
          const SizedBox(height: 12),
          DropdownButtonFormField<String>(
            initialValue: _resourceType,
            decoration: const InputDecoration(labelText: 'Type'),
            items: const [
              DropdownMenuItem(value: 'skill', child: Text('Skill')),
              DropdownMenuItem(value: 'template', child: Text('Template')),
              DropdownMenuItem(value: 'interface', child: Text('Interface')),
            ],
            onChanged: (value) {
              if (value != null) setState(() => _resourceType = value);
            },
          ),
        ],
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('Cancel'),
        ),
        FilledButton(
          onPressed: _isSubmitting ? null : _submit,
          child: _isSubmitting
              ? const SizedBox(
                  width: 16,
                  height: 16,
                  child: CircularProgressIndicator(strokeWidth: 2),
                )
              : const Text('Publish'),
        ),
      ],
    );
  }
}

// ---------------------------------------------------------------------------
// Shared widgets

class _EmptyView extends StatelessWidget {
  const _EmptyView({
    required this.icon,
    required this.title,
    required this.subtitle,
  });

  final IconData icon;
  final String title;
  final String subtitle;

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(32),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(icon, size: 48, color: Colors.grey),
            const SizedBox(height: 16),
            Text(
              title,
              style: const TextStyle(fontSize: 18, fontWeight: FontWeight.w600),
            ),
            const SizedBox(height: 8),
            Text(subtitle, textAlign: TextAlign.center),
          ],
        ),
      ),
    );
  }
}

class _ErrorView extends StatelessWidget {
  const _ErrorView({required this.message, this.onRetry});
  final String message;
  final VoidCallback? onRetry;

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Text(message),
          if (onRetry != null) ...[
            const SizedBox(height: 12),
            FilledButton.tonal(onPressed: onRetry, child: const Text('Retry')),
          ],
        ],
      ),
    );
  }
}
