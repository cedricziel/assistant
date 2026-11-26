import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../connection/connection_provider.dart';
import 'mcp_app_backend.dart';
import 'mcp_app_bridge.dart';
import 'mcp_app_surface.dart';
import 'mcp_app_surface_platform.dart';

/// Renders app frames for the current platform; tests swap in a fake.
final mcpAppSurfaceFactoryProvider = Provider<McpAppSurfaceFactory>(
  (ref) => createPlatformMcpAppSurface,
);

/// The routes an app's calls go to, or `null` while disconnected.
final mcpAppBackendProvider =
    Provider.family<
      McpAppBackend?,
      ({String conversationId, String toolCallId})
    >((ref, call) {
      final api = ref.watch(apiClientProvider);
      if (api == null) return null;
      return ApiMcpAppBackend(
        api.conversations,
        conversationId: call.conversationId,
        toolCallId: call.toolCallId,
      );
    });
