import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../features/chat/chat_screen.dart';
import '../features/connection/connection_provider.dart';
import '../features/connection/connection_screen.dart';
import '../features/logs/logs_screen.dart';
import '../features/skills/skills_screen.dart';
import '../features/traces/traces_screen.dart';

/// Named route constants.
class AppRoutes {
  static const setup = '/setup';
  static const chat = '/chat';
  static const chatConversation = '/chat/:id';
  static const traces = '/traces';
  static const logs = '/logs';
  static const skills = '/skills';
}

/// Build the application router.
///
/// Redirect rules:
/// - Unauthenticated → `/setup`
/// - Authenticated + on `/setup` → `/chat`
GoRouter buildRouter(WidgetRef ref) {
  return GoRouter(
    initialLocation: AppRoutes.chat,
    refreshListenable: _RouterRefreshNotifier(ref),
    redirect: (context, state) {
      final isConnected = ref.read(isConnectedProvider);
      final onSetup = state.fullPath == AppRoutes.setup;

      if (!isConnected && !onSetup) {
        return AppRoutes.setup;
      }
      if (isConnected && onSetup) {
        return AppRoutes.chat;
      }
      return null;
    },
    routes: [
      // -- Connection Setup ---------------------------------------------------
      GoRoute(
        path: AppRoutes.setup,
        builder: (context, state) => const ConnectionScreen(),
      ),

      // -- Chat --------------------------------------------------------------
      GoRoute(
        path: AppRoutes.chat,
        builder: (context, state) => const ChatScreen(),
        routes: [
          GoRoute(
            path: ':id',
            builder: (context, state) {
              final id = state.pathParameters['id'];
              return ChatScreen(conversationId: id);
            },
          ),
        ],
      ),

      // -- Observability: Traces ---------------------------------------------
      GoRoute(
        path: AppRoutes.traces,
        builder: (context, state) => const TracesScreen(),
      ),

      // -- Observability: Logs -----------------------------------------------
      GoRoute(
        path: AppRoutes.logs,
        builder: (context, state) => const LogsScreen(),
      ),

      // -- Skills (read-only discovery) --------------------------------------
      GoRoute(
        path: AppRoutes.skills,
        builder: (context, state) => const SkillsScreen(),
      ),
    ],
  );
}

/// A [ChangeNotifier] that tells go_router to re-evaluate redirects whenever
/// the connection state changes.
class _RouterRefreshNotifier extends ChangeNotifier {
  _RouterRefreshNotifier(WidgetRef ref) {
    ref.listenManual(isConnectedProvider, (prev, next) => notifyListeners());
  }
}
