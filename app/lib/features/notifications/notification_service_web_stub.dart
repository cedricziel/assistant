// Stub for non-web platforms.  All functions are no-ops.

Future<bool> requestWebPermission() async => false;

Future<Map<String, dynamic>?> subscribeToWebPush(String _) async =>
    null;

Future<String?> getCurrentPushEndpoint() async => null;
