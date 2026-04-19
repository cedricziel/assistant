import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../contexts/models/context_model.dart';
import '../contexts/providers/context_providers.dart';
import '../../router/app_router.dart';

/// Web-only login screen.
///
/// Displays the server URL (derived from [Uri.base.origin]) as read-only text
/// and prompts the user for an optional auth token.  On submit it upserts a
/// local context and activates it, then navigates to [AppRoutes.chat].
class LoginScreen extends ConsumerStatefulWidget {
  const LoginScreen({super.key});

  @override
  ConsumerState<LoginScreen> createState() => _LoginScreenState();
}

class _LoginScreenState extends ConsumerState<LoginScreen> {
  final _formKey = GlobalKey<FormState>();
  final _tokenCtrl = TextEditingController();
  bool _tokenVisible = false;
  bool _saving = false;

  // Derive server URL from the page origin at runtime.
  // Uri.base is always available on all platforms; on web it equals
  // window.location.href, so Uri.base.origin is window.location.origin.
  String get _serverUrl {
    final base = Uri.base;
    return '${base.scheme}://${base.host}${base.hasPort && base.port != 80 && base.port != 443 ? ':${base.port}' : ''}';
  }

  String get _serverHost => Uri.base.host;

  @override
  void dispose() {
    _tokenCtrl.dispose();
    super.dispose();
  }

  Future<void> _submit() async {
    if (!_formKey.currentState!.validate()) return;
    setState(() => _saving = true);
    try {
      final token = _tokenCtrl.text.trim();
      final newCtx = AssistantContext.create(
        name: _serverHost,
        serverUrl: _serverUrl,
        authToken: token.isEmpty ? null : token,
      );
      final saved = await ref
          .read(contextsProvider.notifier)
          .upsertContextByUrl(newCtx);
      await ref.read(activeContextProvider.notifier).activate(saved);
      if (mounted) {
        GoRouter.of(context).go(AppRoutes.chat);
      }
    } finally {
      if (mounted) setState(() => _saving = false);
    }
  }

  @override
  Widget build(BuildContext buildContext) {
    return Scaffold(
      body: Center(
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 400),
          child: Padding(
            padding: const EdgeInsets.all(32),
            child: Form(
              key: _formKey,
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  Text(
                    'Sign in',
                    style: Theme.of(buildContext).textTheme.headlineMedium,
                  ),
                  const SizedBox(height: 8),
                  Text(
                    'Enter your API token to connect to this server.',
                    style: Theme.of(buildContext).textTheme.bodyMedium,
                  ),
                  const SizedBox(height: 24),
                  // Read-only server URL.
                  InputDecorator(
                    decoration: const InputDecoration(
                      labelText: 'Server',
                      border: OutlineInputBorder(),
                    ),
                    child: Text(
                      _serverUrl,
                      style: Theme.of(buildContext).textTheme.bodyMedium,
                    ),
                  ),
                  const SizedBox(height: 16),
                  // Token input.
                  TextFormField(
                    controller: _tokenCtrl,
                    decoration: InputDecoration(
                      labelText: 'Token (optional)',
                      border: const OutlineInputBorder(),
                      suffixIcon: IconButton(
                        icon: Icon(
                          _tokenVisible
                              ? Icons.visibility_off
                              : Icons.visibility,
                        ),
                        onPressed: () =>
                            setState(() => _tokenVisible = !_tokenVisible),
                      ),
                    ),
                    obscureText: !_tokenVisible,
                    textInputAction: TextInputAction.done,
                    onFieldSubmitted: (_) => _saving ? null : _submit(),
                  ),
                  const SizedBox(height: 24),
                  FilledButton(
                    onPressed: _saving ? null : _submit,
                    child: _saving
                        ? SizedBox(
                            width: 18,
                            height: 18,
                            child: CircularProgressIndicator.adaptive(
                              strokeWidth: 2,
                              valueColor: AlwaysStoppedAnimation<Color>(
                                Theme.of(buildContext).colorScheme.onPrimary,
                              ),
                            ),
                          )
                        : const Text('Connect'),
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}
