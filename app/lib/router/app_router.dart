import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../features/agents/agent_detail_screen.dart';
import '../features/agents/agents_screen.dart';
import '../features/analytics/analytics_screen.dart';
import '../features/chat/chat_screen.dart';
import '../features/connection/connection_screen.dart';
import '../features/contexts/models/context_model.dart';
import '../features/contexts/providers/context_providers.dart';
import '../features/contexts/screens/context_switcher_screen.dart';
import '../features/logs/logs_screen.dart';
import '../features/personas/persona_create_screen.dart';
import '../features/personas/persona_detail_screen.dart';
import '../features/personas/persona_file_editor_screen.dart';
import '../features/personas/personas_screen.dart';
import '../features/settings/settings_screen.dart';
import '../features/skills/skill_create_edit_screen.dart';
import '../features/skills/skill_detail_screen.dart';
import '../features/skills/skills_screen.dart';
import '../features/traces/trace_detail_screen.dart';
import '../features/traces/traces_screen.dart';
import '../features/webhooks/webhook_detail_screen.dart';
import '../features/webhooks/webhooks_screen.dart';
import '../features/workflows/workflow_detail_screen.dart';
import '../features/workflows/workflow_editor_screen.dart';
import '../features/workflows/workflow_run_detail_screen.dart';
import '../features/workflows/workflows_screen.dart';
import '../shared/loading_screen.dart';
import '../shared/nav_shell.dart';

/// Named route constants.
class AppRoutes {
  static const loading = '/loading';
  static const setup = '/setup';
  static const contexts = '/contexts';
  static const chat = '/chat';
  static const chatConversation = '/chat/:id';
  static const traces = '/traces';
  static const traceDetail = '/traces/:traceId';
  static const logs = '/logs';
  static const skills = '/skills';
  static const skillNew = '/skills/new';
  static const skillDetail = '/skills/:name';
  static const skillEdit = '/skills/:name/edit';
  static const personas = '/personas';
  static const personasNew = '/personas/new';
  static const personasDetail = '/personas/:id';
  static const personasFile = '/personas/:id/files/:filename';
  static const workflows = '/workflows';
  static const workflowNew = '/workflows/new';
  static const workflowDetail = '/workflows/:id';
  static const workflowEdit = '/workflows/:id/edit';
  static const workflowRunDetail = '/workflows/:id/runs/:runId';
  static const webhooks = '/webhooks';
  static const webhookDetail = '/webhooks/:id';
  static const agents = '/agents';
  static const agentDetail = '/agents/:id';
  static const analytics = '/analytics';
  static const settings = '/settings';
}

/// Provider that creates a single [GoRouter] instance for the application
/// lifetime. Creating this as a [Provider] ensures the router is not
/// re-instantiated on every widget rebuild, which would lose navigation state.
final routerProvider = Provider<GoRouter>((ref) {
  final notifier = _RouterRefreshNotifier(ref);
  ref.onDispose(notifier.dispose);
  return GoRouter(
    initialLocation: AppRoutes.chat,
    refreshListenable: notifier,
    redirect: (context, state) {
      final activeContextAsync = ref.read(activeContextProvider);

      // While the context is still loading from storage, show the loading
      // scaffold — the router will re-evaluate once the AsyncNotifier settles.
      if (activeContextAsync.isLoading) return AppRoutes.loading;

      final hasContext = activeContextAsync.value != null;
      final onContextSwitcher = state.fullPath == AppRoutes.contexts;
      final onSetup = state.fullPath == AppRoutes.setup;
      final onLoading = state.fullPath == AppRoutes.loading;

      // Once loading is done, navigate away from the loading scaffold to
      // the correct destination.
      if (onLoading) {
        return hasContext ? AppRoutes.chat : AppRoutes.contexts;
      }

      // If no active context and not already on the switcher or setup,
      // redirect to the context switcher.
      if (!hasContext && !onContextSwitcher && !onSetup) {
        return AppRoutes.contexts;
      }
      return null;
    },
    routes: [
      // -- Loading scaffold: shown while async providers initialise -----------
      GoRoute(
        path: AppRoutes.loading,
        builder: (context, state) => const LoadingScreen(),
      ),

      // -- Legacy setup route (kept for deep-link compatibility) -------------
      GoRoute(
        path: AppRoutes.setup,
        builder: (context, state) => const ConnectionScreen(),
      ),

      // -- Shell: all main screens use the navigation shell ------------------
      ShellRoute(
        builder: (context, state, child) => NavShell(child: child),
        routes: [
          // -- Context Switcher ----------------------------------------------
          GoRoute(
            path: AppRoutes.contexts,
            builder: (context, state) => const ContextSwitcherScreen(),
          ),

          // -- Chat ----------------------------------------------------------
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

          // -- Observability: Traces -----------------------------------------
          GoRoute(
            path: AppRoutes.traces,
            builder: (context, state) => const TracesScreen(),
            routes: [
              GoRoute(
                path: ':traceId',
                builder: (context, state) {
                  final traceId = state.pathParameters['traceId']!;
                  return TraceDetailScreen(traceId: traceId);
                },
              ),
            ],
          ),

          // -- Observability: Logs -------------------------------------------
          GoRoute(
            path: AppRoutes.logs,
            builder: (context, state) => const LogsScreen(),
          ),

          // -- Skills --------------------------------------------------------
          GoRoute(
            path: AppRoutes.skills,
            builder: (context, state) => const SkillsScreen(),
            routes: [
              GoRoute(
                path: 'new',
                builder: (context, state) => const SkillCreateEditScreen(),
              ),
              GoRoute(
                path: ':name',
                builder: (context, state) {
                  final name = state.pathParameters['name']!;
                  return SkillDetailScreen(skillName: name);
                },
                routes: [
                  GoRoute(
                    path: 'edit',
                    builder: (context, state) {
                      final name = state.pathParameters['name']!;
                      return SkillCreateEditScreen(skillName: name);
                    },
                  ),
                ],
              ),
            ],
          ),

          // -- Personas -------------------------------------------------------
          GoRoute(
            path: AppRoutes.personas,
            builder: (context, state) => const PersonasScreen(),
            routes: [
              GoRoute(
                path: 'new',
                builder: (context, state) => const PersonaCreateScreen(),
              ),
              GoRoute(
                path: ':id',
                builder: (context, state) {
                  final id = state.pathParameters['id']!;
                  return PersonaDetailScreen(personaId: id);
                },
                routes: [
                  GoRoute(
                    path: 'files/:filename',
                    builder: (context, state) {
                      final id = state.pathParameters['id']!;
                      final filename = state.pathParameters['filename']!;
                      return PersonaFileEditorScreen(
                        personaId: id,
                        filename: filename,
                      );
                    },
                  ),
                ],
              ),
            ],
          ),

          // -- Workflows -----------------------------------------------------
          GoRoute(
            path: AppRoutes.workflows,
            builder: (context, state) => const WorkflowsScreen(),
            routes: [
              GoRoute(
                path: 'new',
                builder: (context, state) => const WorkflowEditorScreen(),
              ),
              GoRoute(
                path: ':id',
                builder: (context, state) {
                  final id = state.pathParameters['id']!;
                  return WorkflowDetailScreen(workflowId: id);
                },
                routes: [
                  GoRoute(
                    path: 'edit',
                    builder: (context, state) {
                      final id = state.pathParameters['id']!;
                      return WorkflowEditorScreen(workflowId: id);
                    },
                  ),
                  GoRoute(
                    path: 'runs/:runId',
                    builder: (context, state) {
                      final id = state.pathParameters['id']!;
                      final runId = state.pathParameters['runId']!;
                      return WorkflowRunDetailScreen(
                        workflowId: id,
                        runId: runId,
                      );
                    },
                  ),
                ],
              ),
            ],
          ),

          // -- Webhooks ------------------------------------------------------
          GoRoute(
            path: AppRoutes.webhooks,
            builder: (context, state) => const WebhooksScreen(),
            routes: [
              GoRoute(
                path: ':id',
                builder: (context, state) {
                  final id = state.pathParameters['id']!;
                  return WebhookDetailScreen(webhookId: id);
                },
              ),
            ],
          ),

          // -- Agents -------------------------------------------------------
          GoRoute(
            path: AppRoutes.agents,
            builder: (context, state) => const AgentsScreen(),
            routes: [
              GoRoute(
                path: ':id',
                builder: (context, state) {
                  final id = state.pathParameters['id']!;
                  return AgentDetailScreen(agentId: id);
                },
              ),
            ],
          ),

          // -- Analytics ----------------------------------------------------
          GoRoute(
            path: AppRoutes.analytics,
            builder: (context, state) => const AnalyticsScreen(),
          ),

          // -- Settings -----------------------------------------------------
          GoRoute(
            path: AppRoutes.settings,
            builder: (context, state) => const SettingsScreen(),
          ),
        ],
      ),
    ],
  );
});

/// A [ChangeNotifier] that tells go_router to re-evaluate redirects whenever
/// the active context changes.
class _RouterRefreshNotifier extends ChangeNotifier {
  _RouterRefreshNotifier(Ref ref) {
    // Listen to the raw async state so the router re-evaluates on every
    // transition (loading→data, data→data, etc.), not just bool changes.
    _sub = ref.listen<AsyncValue<AssistantContext?>>(
      activeContextProvider,
      (prev, next) => notifyListeners(),
    );
  }

  late final ProviderSubscription<AsyncValue<AssistantContext?>> _sub;

  @override
  void dispose() {
    _sub.close();
    super.dispose();
  }
}
