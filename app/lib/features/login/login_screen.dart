import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../router/app_router.dart';
import '../../shared/platform/platform.dart';
import '../auth/auth_provider.dart';

/// Login mode shown in the UI.
enum _LoginMode { oauth2, legacyToken }

/// Adaptive login screen supporting both OAuth2 (email + password) and
/// legacy token authentication.
///
/// On web, the server URL is derived from the page origin. On native
/// platforms, the user enters the server URL manually.
class LoginScreen extends ConsumerStatefulWidget {
  const LoginScreen({super.key});

  @override
  ConsumerState<LoginScreen> createState() => _LoginScreenState();
}

class _LoginScreenState extends ConsumerState<LoginScreen> {
  final _formKey = GlobalKey<FormState>();
  final _serverCtrl = TextEditingController();
  final _emailCtrl = TextEditingController();
  final _passwordCtrl = TextEditingController();
  final _tokenCtrl = TextEditingController();
  bool _passwordVisible = false;
  bool _tokenVisible = false;
  _LoginMode _mode = _LoginMode.oauth2;

  String get _serverUrl {
    if (isMaterial && _isWebContext) {
      // On web, derive from page origin.
      final base = Uri.base;
      return '${base.scheme}://${base.host}'
          '${base.hasPort && base.port != 80 && base.port != 443 ? ':${base.port}' : ''}';
    }
    return _serverCtrl.text.trim();
  }

  bool get _isWebContext {
    try {
      // Uri.base.host is empty in test environments.
      return Uri.base.host.isNotEmpty;
    } catch (_) {
      return false;
    }
  }

  @override
  void dispose() {
    _serverCtrl.dispose();
    _emailCtrl.dispose();
    _passwordCtrl.dispose();
    _tokenCtrl.dispose();
    super.dispose();
  }

  Future<void> _submit() async {
    if (!_formKey.currentState!.validate()) return;

    final loginNotifier = ref.read(oauthLoginProvider.notifier);

    if (_mode == _LoginMode.oauth2) {
      await loginNotifier.login(
        serverUrl: _serverUrl,
        email: _emailCtrl.text.trim(),
        password: _passwordCtrl.text,
      );
    } else {
      final token = _tokenCtrl.text.trim();
      await loginNotifier.legacyLogin(
        serverUrl: _serverUrl,
        token: token.isEmpty ? null : token,
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    final loginState = ref.watch(oauthLoginProvider);
    final isLoading = loginState is OAuthLoginLoading;
    final loadingMessage = loginState is OAuthLoginLoading
        ? loginState.message
        : null;

    // Navigate on success.
    ref.listen<OAuthLoginState>(oauthLoginProvider, (prev, next) {
      if (next is OAuthLoginSuccess && mounted) {
        GoRouter.of(this.context).go(AppRoutes.chat);
      }
    });

    return Scaffold(
      body: Center(
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 420),
          child: SingleChildScrollView(
            padding: const EdgeInsets.all(32),
            child: Form(
              key: _formKey,
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  Text(
                    'Sign in',
                    style: Theme.of(context).textTheme.headlineMedium,
                  ),
                  const SizedBox(height: 8),
                  Text(
                    _mode == _LoginMode.oauth2
                        ? 'Sign in with your email and password.'
                        : 'Enter an API token to connect.',
                    style: Theme.of(context).textTheme.bodyMedium,
                  ),
                  const SizedBox(height: 24),

                  // Server URL — read-only on web, editable on native.
                  if (_isWebContext)
                    InputDecorator(
                      decoration: const InputDecoration(
                        labelText: 'Server',
                        border: OutlineInputBorder(),
                      ),
                      child: Text(
                        _serverUrl,
                        style: Theme.of(context).textTheme.bodyMedium,
                      ),
                    )
                  else
                    TextFormField(
                      controller: _serverCtrl,
                      decoration: const InputDecoration(
                        labelText: 'Server URL',
                        hintText: 'http://localhost:8080',
                        border: OutlineInputBorder(),
                      ),
                      keyboardType: TextInputType.url,
                      textInputAction: TextInputAction.next,
                      validator: (v) {
                        if (v == null || v.trim().isEmpty) {
                          return 'Server URL is required';
                        }
                        final uri = Uri.tryParse(v.trim());
                        if (uri == null || !uri.hasScheme) {
                          return 'Enter a valid URL (e.g. http://localhost:8080)';
                        }
                        return null;
                      },
                    ),
                  const SizedBox(height: 16),

                  // Auth mode fields.
                  if (_mode == _LoginMode.oauth2) ...[
                    TextFormField(
                      controller: _emailCtrl,
                      decoration: const InputDecoration(
                        labelText: 'Email',
                        border: OutlineInputBorder(),
                      ),
                      keyboardType: TextInputType.emailAddress,
                      textInputAction: TextInputAction.next,
                      autofillHints: const [AutofillHints.email],
                      validator: (v) {
                        if (v == null || v.trim().isEmpty) {
                          return 'Email is required';
                        }
                        return null;
                      },
                    ),
                    const SizedBox(height: 16),
                    TextFormField(
                      controller: _passwordCtrl,
                      decoration: InputDecoration(
                        labelText: 'Password',
                        border: const OutlineInputBorder(),
                        suffixIcon: IconButton(
                          icon: Icon(
                            _passwordVisible
                                ? Icons.visibility_off
                                : Icons.visibility,
                          ),
                          onPressed: () => setState(
                            () => _passwordVisible = !_passwordVisible,
                          ),
                        ),
                      ),
                      obscureText: !_passwordVisible,
                      textInputAction: TextInputAction.done,
                      autofillHints: const [AutofillHints.password],
                      validator: (v) {
                        if (v == null || v.isEmpty) {
                          return 'Password is required';
                        }
                        return null;
                      },
                      onFieldSubmitted: (_) => isLoading ? null : _submit(),
                    ),
                  ] else ...[
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
                      onFieldSubmitted: (_) => isLoading ? null : _submit(),
                    ),
                  ],

                  // Error message.
                  if (loginState is OAuthLoginError) ...[
                    const SizedBox(height: 16),
                    Container(
                      padding: const EdgeInsets.all(12),
                      decoration: BoxDecoration(
                        color: Theme.of(context).colorScheme.errorContainer,
                        borderRadius: BorderRadius.circular(8),
                      ),
                      child: Text(
                        loginState.message,
                        style: TextStyle(
                          color: Theme.of(context).colorScheme.onErrorContainer,
                        ),
                      ),
                    ),
                  ],

                  const SizedBox(height: 24),

                  // Submit button.
                  FilledButton(
                    onPressed: isLoading ? null : _submit,
                    child: isLoading
                        ? Row(
                            mainAxisSize: MainAxisSize.min,
                            children: [
                              SizedBox(
                                width: 18,
                                height: 18,
                                child: CircularProgressIndicator.adaptive(
                                  strokeWidth: 2,
                                  valueColor: AlwaysStoppedAnimation<Color>(
                                    Theme.of(context).colorScheme.onPrimary,
                                  ),
                                ),
                              ),
                              if (loadingMessage != null) ...[
                                const SizedBox(width: 12),
                                Text(loadingMessage),
                              ],
                            ],
                          )
                        : Text(
                            _mode == _LoginMode.oauth2 ? 'Sign in' : 'Connect',
                          ),
                  ),

                  const SizedBox(height: 16),

                  // Mode switcher.
                  Center(
                    child: TextButton(
                      onPressed: isLoading
                          ? null
                          : () => setState(() {
                              _mode = _mode == _LoginMode.oauth2
                                  ? _LoginMode.legacyToken
                                  : _LoginMode.oauth2;
                            }),
                      child: Text(
                        _mode == _LoginMode.oauth2
                            ? 'Use token authentication instead'
                            : 'Sign in with email and password',
                      ),
                    ),
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
