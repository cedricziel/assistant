import 'package:material_ui/material_ui.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../router/app_router.dart';
import '../auth/auth_provider.dart';
import '../auth/browser_sign_in.dart';

/// Login mode shown in the UI.
enum _LoginMode { oauth2, legacyToken }

/// Login screen: OAuth2 sign-in on the server's own sign-in page, or legacy
/// token authentication.
///
/// The server's page asks for the email, finds the org and shows its
/// password form or identity provider (OIDC). On web the server URL is the
/// page origin; on native platforms the user enters it.
class LoginScreen extends ConsumerStatefulWidget {
  const LoginScreen({super.key});

  @override
  ConsumerState<LoginScreen> createState() => _LoginScreenState();
}

class _LoginScreenState extends ConsumerState<LoginScreen> {
  final _formKey = GlobalKey<FormState>();
  final _serverCtrl = TextEditingController();
  final _tokenCtrl = TextEditingController();
  // Explicit focus nodes so Tab/Shift+Tab traversal is deterministic and
  // `onFieldSubmitted` (Enter) advances correctly on macOS desktop.
  final _serverFocus = FocusNode(debugLabel: 'login.server');
  final _tokenFocus = FocusNode(debugLabel: 'login.token');
  bool _tokenVisible = false;
  _LoginMode _mode = _LoginMode.oauth2;
  bool _bannerDismissed = false;

  String? get _hostedServerUrl =>
      ref.read(browserSignInPlatformProvider).hostedServerUrl;

  String get _serverUrl => _hostedServerUrl ?? _serverCtrl.text.trim();

  @override
  void dispose() {
    _serverCtrl.dispose();
    _tokenCtrl.dispose();
    _serverFocus.dispose();
    _tokenFocus.dispose();
    super.dispose();
  }

  Future<void> _submit() async {
    if (!_formKey.currentState!.validate()) return;

    final loginNotifier = ref.read(oauthLoginProvider.notifier);

    if (_mode == _LoginMode.oauth2) {
      await loginNotifier.startBrowserSignIn(serverUrl: _serverUrl);
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

    final reason = GoRouterState.of(context).uri.queryParameters['reason'];
    final showSessionEndedBanner =
        reason == 'session-ended' && !_bannerDismissed;

    return Scaffold(
      body: Center(
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 420),
          child: SingleChildScrollView(
            padding: const EdgeInsets.all(32),
            child: AutofillGroup(
              child: Form(
                key: _formKey,
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    if (showSessionEndedBanner) ...[
                      _SessionEndedBanner(
                        onDismiss: () =>
                            setState(() => _bannerDismissed = true),
                      ),
                      const SizedBox(height: 16),
                    ],
                    Text(
                      'Sign in',
                      style: Theme.of(context).textTheme.headlineMedium,
                    ),
                    const SizedBox(height: 8),
                    Text(
                      _mode == _LoginMode.oauth2
                          ? "You'll continue on your server's sign-in page."
                          : 'Enter an API token to connect.',
                      style: Theme.of(context).textTheme.bodyMedium,
                    ),
                    const SizedBox(height: 24),

                    // Server URL — read-only on web, editable on native.
                    if (_hostedServerUrl case final hosted?)
                      InputDecorator(
                        decoration: const InputDecoration(
                          labelText: 'Server',
                          border: OutlineInputBorder(),
                        ),
                        child: Text(
                          hosted,
                          style: Theme.of(context).textTheme.bodyMedium,
                        ),
                      )
                    else
                      TextFormField(
                        controller: _serverCtrl,
                        focusNode: _serverFocus,
                        decoration: const InputDecoration(
                          labelText: 'Server URL',
                          hintText: 'http://localhost:8080',
                          border: OutlineInputBorder(),
                        ),
                        keyboardType: TextInputType.url,
                        textInputAction: _mode == _LoginMode.oauth2
                            ? TextInputAction.done
                            : TextInputAction.next,
                        autofillHints: const [AutofillHints.url],
                        onFieldSubmitted: (_) => _mode == _LoginMode.oauth2
                            ? (isLoading ? null : _submit())
                            : _tokenFocus.requestFocus(),
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

                    // Token field.
                    if (_mode == _LoginMode.legacyToken) ...[
                      TextFormField(
                        controller: _tokenCtrl,
                        focusNode: _tokenFocus,
                        autofocus: true,
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
                            color: Theme.of(context)
                                .colorScheme
                                .onErrorContainer,
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
                                  Flexible(
                                    child: Text(
                                      loadingMessage,
                                      overflow: TextOverflow.ellipsis,
                                    ),
                                  ),
                                ],
                              ],
                            )
                          : Text(
                              _mode == _LoginMode.oauth2
                                  ? 'Sign in'
                                  : 'Connect',
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
                              : 'Sign in with your account',
                        ),
                      ),
                    ),
                  ],
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}

/// Banner shown on `/login?reason=session-ended` to explain why the user was
/// bounced back to the login screen — typically because the browser wiped
/// the secure-storage crypto key.
class _SessionEndedBanner extends StatelessWidget {
  const _SessionEndedBanner({required this.onDismiss});

  final VoidCallback onDismiss;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Container(
      key: const Key('session-ended-banner'),
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: colorScheme.errorContainer,
        borderRadius: BorderRadius.circular(8),
      ),
      child: Row(
        children: [
          Icon(Icons.info_outline, color: colorScheme.onErrorContainer),
          const SizedBox(width: 12),
          Expanded(
            child: Text(
              'Your session was reset by the browser. Please log in again.',
              style: TextStyle(color: colorScheme.onErrorContainer),
            ),
          ),
          IconButton(
            key: const Key('session-ended-banner-dismiss'),
            tooltip: 'Dismiss',
            icon: Icon(Icons.close, color: colorScheme.onErrorContainer),
            onPressed: onDismiss,
          ),
        ],
      ),
    );
  }
}
