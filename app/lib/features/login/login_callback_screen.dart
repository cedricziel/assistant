import 'package:material_ui/material_ui.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../router/app_router.dart';
import '../auth/auth_provider.dart';

/// Landing route for the server's sign-in redirect: exchanges the returned
/// authorization code for tokens, then continues into the app.
class LoginCallbackScreen extends ConsumerStatefulWidget {
  const LoginCallbackScreen({super.key, required this.query});

  /// Query parameters of the redirect (`code`, `state`, or `error`).
  final Map<String, String> query;

  @override
  ConsumerState<LoginCallbackScreen> createState() =>
      _LoginCallbackScreenState();
}

class _LoginCallbackScreenState extends ConsumerState<LoginCallbackScreen> {
  @override
  void initState() {
    super.initState();
    // Providers must not change while the tree is building.
    WidgetsBinding.instance.addPostFrameCallback((_) {
      ref.read(oauthLoginProvider.notifier).completeBrowserSignIn(widget.query);
    });
  }

  @override
  Widget build(BuildContext context) {
    ref.listen<OAuthLoginState>(oauthLoginProvider, (prev, next) {
      if (next is OAuthLoginSuccess && mounted) {
        GoRouter.of(context).go(AppRoutes.chat);
      }
    });

    final loginState = ref.watch(oauthLoginProvider);

    return Scaffold(
      body: Center(
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 420),
          child: Padding(
            padding: const EdgeInsets.all(32),
            child: loginState is OAuthLoginError
                ? Column(
                    mainAxisSize: MainAxisSize.min,
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: [
                      Text(
                        loginState.message,
                        style: TextStyle(
                          color: Theme.of(context).colorScheme.error,
                        ),
                      ),
                      const SizedBox(height: 24),
                      FilledButton(
                        onPressed: () =>
                            GoRouter.of(context).go(AppRoutes.login),
                        child: const Text('Back to sign in'),
                      ),
                    ],
                  )
                : const Center(child: CircularProgressIndicator.adaptive()),
          ),
        ),
      ),
    );
  }
}
