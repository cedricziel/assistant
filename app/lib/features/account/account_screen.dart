import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'account_provider.dart';

/// Self-service account screen — view/edit name, email, and password.
class AccountScreen extends ConsumerWidget {
  const AccountScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final state = ref.watch(accountProvider);
    return Scaffold(
      appBar: AppBar(title: const Text('Account')),
      body: state.when(
        loading: () =>
            const Center(child: CircularProgressIndicator.adaptive()),
        error: (err, _) => Center(
          child: Padding(
            padding: const EdgeInsets.all(24),
            child: Text('Failed to load account: $err'),
          ),
        ),
        data: (account) => _AccountBody(account: account),
      ),
    );
  }
}

class _AccountBody extends StatelessWidget {
  const _AccountBody({required this.account});

  final AccountState account;

  @override
  Widget build(BuildContext context) {
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        if (account.isOidc) _OidcBanner(issuer: account.issuer),
        const SizedBox(height: 8),
        _NameSection(account: account),
        const SizedBox(height: 24),
        _EmailSection(account: account),
        const SizedBox(height: 24),
        if (!account.isOidc) const _PasswordSection(),
      ],
    );
  }
}

class _OidcBanner extends StatelessWidget {
  const _OidcBanner({this.issuer});

  final String? issuer;

  @override
  Widget build(BuildContext context) {
    final issuerLabel = issuer ?? 'your identity provider';
    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: Theme.of(context).colorScheme.surfaceContainerHighest,
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: Theme.of(context).colorScheme.outlineVariant),
      ),
      child: Row(
        children: [
          const Icon(Icons.shield_outlined),
          const SizedBox(width: 12),
          Expanded(
            child: Text(
              'Managed by your identity provider ($issuerLabel). '
              'To change your name, email, or password, sign in there.',
            ),
          ),
        ],
      ),
    );
  }
}

class _NameSection extends ConsumerStatefulWidget {
  const _NameSection({required this.account});
  final AccountState account;

  @override
  ConsumerState<_NameSection> createState() => _NameSectionState();
}

class _NameSectionState extends ConsumerState<_NameSection> {
  late final TextEditingController _ctrl;
  bool _saving = false;

  @override
  void initState() {
    super.initState();
    _ctrl = TextEditingController(text: widget.account.user.name);
  }

  @override
  void dispose() {
    _ctrl.dispose();
    super.dispose();
  }

  Future<void> _save() async {
    setState(() => _saving = true);
    try {
      await ref.read(accountProvider.notifier).updateName(_ctrl.text.trim());
      if (!mounted) return;
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(const SnackBar(content: Text('Name updated')));
    } catch (e) {
      if (!mounted) return;
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text('Failed: $e')));
    } finally {
      if (mounted) setState(() => _saving = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final readOnly = widget.account.isOidc;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Text('Name', style: Theme.of(context).textTheme.titleMedium),
        const SizedBox(height: 8),
        TextField(
          key: const Key('account.name.field'),
          controller: _ctrl,
          readOnly: readOnly,
          decoration: const InputDecoration(border: OutlineInputBorder()),
        ),
        const SizedBox(height: 8),
        if (!readOnly)
          Align(
            alignment: Alignment.centerRight,
            child: FilledButton(
              key: const Key('account.name.save'),
              onPressed: _saving ? null : _save,
              child: _saving
                  ? const SizedBox(
                      width: 16,
                      height: 16,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    )
                  : const Text('Save name'),
            ),
          ),
      ],
    );
  }
}

class _EmailSection extends ConsumerStatefulWidget {
  const _EmailSection({required this.account});
  final AccountState account;

  @override
  ConsumerState<_EmailSection> createState() => _EmailSectionState();
}

class _EmailSectionState extends ConsumerState<_EmailSection> {
  late final TextEditingController _newCtrl;
  late final TextEditingController _confirmCtrl;
  String? _error;
  bool _saving = false;

  @override
  void initState() {
    super.initState();
    _newCtrl = TextEditingController();
    _confirmCtrl = TextEditingController();
  }

  @override
  void dispose() {
    _newCtrl.dispose();
    _confirmCtrl.dispose();
    super.dispose();
  }

  Future<void> _save() async {
    final newEmail = _newCtrl.text.trim();
    final confirm = _confirmCtrl.text.trim();
    if (newEmail.isEmpty) {
      setState(() => _error = 'Enter a new email');
      return;
    }
    if (newEmail != confirm) {
      setState(() => _error = 'Emails do not match');
      return;
    }
    setState(() {
      _error = null;
      _saving = true;
    });
    try {
      await ref.read(accountProvider.notifier).updateEmail(newEmail);
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text('Email changed (was ${widget.account.user.email})'),
        ),
      );
      _newCtrl.clear();
      _confirmCtrl.clear();
    } catch (e) {
      if (!mounted) return;
      setState(() => _error = '$e');
    } finally {
      if (mounted) setState(() => _saving = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final readOnly = widget.account.isOidc;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Text('Email', style: Theme.of(context).textTheme.titleMedium),
        const SizedBox(height: 8),
        TextField(
          readOnly: true,
          controller: TextEditingController(text: widget.account.user.email),
          decoration: const InputDecoration(
            labelText: 'Current',
            border: OutlineInputBorder(),
          ),
        ),
        if (!readOnly) ...[
          const SizedBox(height: 8),
          TextField(
            key: const Key('account.email.new'),
            controller: _newCtrl,
            keyboardType: TextInputType.emailAddress,
            decoration: const InputDecoration(
              labelText: 'New email',
              border: OutlineInputBorder(),
            ),
          ),
          const SizedBox(height: 8),
          TextField(
            key: const Key('account.email.confirm'),
            controller: _confirmCtrl,
            keyboardType: TextInputType.emailAddress,
            decoration: const InputDecoration(
              labelText: 'Confirm new email',
              border: OutlineInputBorder(),
            ),
          ),
          if (_error != null) ...[
            const SizedBox(height: 8),
            Text(
              _error!,
              style: TextStyle(color: Theme.of(context).colorScheme.error),
            ),
          ],
          const SizedBox(height: 8),
          Align(
            alignment: Alignment.centerRight,
            child: FilledButton(
              key: const Key('account.email.save'),
              onPressed: _saving ? null : _save,
              child: _saving
                  ? const SizedBox(
                      width: 16,
                      height: 16,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    )
                  : const Text('Save email'),
            ),
          ),
        ],
      ],
    );
  }
}

class _PasswordSection extends ConsumerStatefulWidget {
  const _PasswordSection();

  @override
  ConsumerState<_PasswordSection> createState() => _PasswordSectionState();
}

class _PasswordSectionState extends ConsumerState<_PasswordSection> {
  final _currentCtrl = TextEditingController();
  final _newCtrl = TextEditingController();
  final _confirmCtrl = TextEditingController();
  String? _error;
  bool _saving = false;

  @override
  void dispose() {
    _currentCtrl.dispose();
    _newCtrl.dispose();
    _confirmCtrl.dispose();
    super.dispose();
  }

  Future<void> _save() async {
    final current = _currentCtrl.text;
    final next = _newCtrl.text;
    final confirm = _confirmCtrl.text;
    if (current.isEmpty || next.isEmpty) {
      setState(() => _error = 'Enter current and new password');
      return;
    }
    if (next != confirm) {
      setState(() => _error = 'New passwords do not match');
      return;
    }
    setState(() {
      _error = null;
      _saving = true;
    });
    try {
      await ref
          .read(accountProvider.notifier)
          .changePassword(currentPassword: current, newPassword: next);
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(
          content: Text('Password changed. Other devices will be signed out.'),
        ),
      );
      _currentCtrl.clear();
      _newCtrl.clear();
      _confirmCtrl.clear();
    } catch (e) {
      if (!mounted) return;
      setState(() => _error = '$e');
    } finally {
      if (mounted) setState(() => _saving = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Text('Password', style: Theme.of(context).textTheme.titleMedium),
        const SizedBox(height: 8),
        TextField(
          key: const Key('account.password.current'),
          controller: _currentCtrl,
          obscureText: true,
          decoration: const InputDecoration(
            labelText: 'Current password',
            border: OutlineInputBorder(),
          ),
        ),
        const SizedBox(height: 8),
        TextField(
          key: const Key('account.password.new'),
          controller: _newCtrl,
          obscureText: true,
          decoration: const InputDecoration(
            labelText: 'New password',
            border: OutlineInputBorder(),
          ),
        ),
        const SizedBox(height: 8),
        TextField(
          key: const Key('account.password.confirm'),
          controller: _confirmCtrl,
          obscureText: true,
          decoration: const InputDecoration(
            labelText: 'Confirm new password',
            border: OutlineInputBorder(),
          ),
        ),
        if (_error != null) ...[
          const SizedBox(height: 8),
          Text(
            _error!,
            style: TextStyle(color: Theme.of(context).colorScheme.error),
          ),
        ],
        const SizedBox(height: 8),
        Align(
          alignment: Alignment.centerRight,
          child: FilledButton(
            key: const Key('account.password.save'),
            onPressed: _saving ? null : _save,
            child: _saving
                ? const SizedBox(
                    width: 16,
                    height: 16,
                    child: CircularProgressIndicator(strokeWidth: 2),
                  )
                : const Text('Change password'),
          ),
        ),
      ],
    );
  }
}
