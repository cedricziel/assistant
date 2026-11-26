import 'package:assistant_ui/models.dart';
import 'package:material_ui/material_ui.dart';

import 'mcp_app_bridge.dart';

/// One rendered app document and the port that talks to it.
abstract interface class McpAppSurface {
  McpAppPort get port;

  Widget build(BuildContext context);

  void dispose();
}

/// Renders [app] in a sandbox on the current platform.
typedef McpAppSurfaceFactory = McpAppSurface Function(McpApp app);
