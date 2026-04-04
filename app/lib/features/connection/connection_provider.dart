import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:http/http.dart' as http;

import '../../api/models/server_profile.dart';

const _kProfileKey = 'assistant_server_profile';

/// State held by [ServerProfileNotifier].
class ServerConnectionState {
  const ServerConnectionState({
    this.profile,
    this.isConnecting = false,
    this.error,
  });

  /// The saved (validated) server profile.  `null` means no profile yet.
  final ServerProfile? profile;

  final bool isConnecting;

  /// Last connection error message, if any.
  final String? error;

  bool get isConnected => profile != null;

  ServerConnectionState copyWith({
    ServerProfile? profile,
    bool? isConnecting,
    String? error,
    bool clearProfile = false,
    bool clearError = false,
  }) {
    return ServerConnectionState(
      profile: clearProfile ? null : (profile ?? this.profile),
      isConnecting: isConnecting ?? this.isConnecting,
      error: clearError ? null : (error ?? this.error),
    );
  }
}

/// Notifier that manages the server connection profile.
///
/// Persists credentials in [flutter_secure_storage] (platform keychain).
/// Validates connections via `GET /health` before saving.
class ServerProfileNotifier extends AsyncNotifier<ServerConnectionState> {
  static const _storage = FlutterSecureStorage();

  @override
  Future<ServerConnectionState> build() async {
    return _loadProfile();
  }

  // -- Load from secure storage -----------------------------------------------

  Future<ServerConnectionState> _loadProfile() async {
    try {
      final raw = await _storage.read(key: _kProfileKey);
      if (raw == null || raw.isEmpty) {
        return const ServerConnectionState();
      }
      final profile = ServerProfile.fromJson(raw);
      return ServerConnectionState(profile: profile);
    } catch (_) {
      return const ServerConnectionState();
    }
  }

  // -- Connect ----------------------------------------------------------------

  /// Validate and save a server connection.
  ///
  /// 1. Calls `GET <baseUrl>/health` to verify the server is reachable.
  /// 2. If the server returns 401, throws `"Invalid token"`.
  /// 3. On network failure, throws `"Server unreachable"`.
  /// 4. On success, persists the profile and updates state.
  Future<void> connect(String baseUrl, String token) async {
    state = AsyncData(
      (state.valueOrNull ?? const ServerConnectionState()).copyWith(
        isConnecting: true,
        clearError: true,
      ),
    );

    try {
      final trimmedUrl = baseUrl.trim().replaceAll(RegExp(r'/$'), '');
      final trimmedToken = token.trim();

      // Validate: ping the health endpoint.
      final healthUri = Uri.parse('$trimmedUrl/health');
      final response = await http
          .get(
            healthUri,
            headers: {'Authorization': 'Bearer $trimmedToken'},
          )
          .timeout(const Duration(seconds: 10));

      if (response.statusCode == 401) {
        state = AsyncData(
          (state.valueOrNull ?? const ServerConnectionState()).copyWith(
            isConnecting: false,
            error: 'Invalid token',
          ),
        );
        return;
      }

      if (response.statusCode != 200) {
        state = AsyncData(
          (state.valueOrNull ?? const ServerConnectionState()).copyWith(
            isConnecting: false,
            error: 'Server returned ${response.statusCode}',
          ),
        );
        return;
      }

      // Connection valid — save and update state.
      final profile = ServerProfile(
        baseUrl: trimmedUrl,
        token: trimmedToken,
      );
      await _storage.write(key: _kProfileKey, value: profile.toJson());

      state = AsyncData(
        ServerConnectionState(profile: profile),
      );
    } on Exception catch (e) {
      final message = e.toString().contains('Connection refused') ||
              e.toString().contains('Failed host lookup') ||
              e.toString().contains('TimeoutException')
          ? 'Server unreachable'
          : e.toString();
      state = AsyncData(
        (state.valueOrNull ?? const ServerConnectionState()).copyWith(
          isConnecting: false,
          error: message,
        ),
      );
    }
  }

  // -- Disconnect -------------------------------------------------------------

  /// Clear stored credentials and return to the unauthenticated state.
  Future<void> disconnect() async {
    await _storage.delete(key: _kProfileKey);
    state = const AsyncData(ServerConnectionState());
  }
}

/// Provider for the [ServerProfileNotifier].
final serverProfileProvider =
    AsyncNotifierProvider<ServerProfileNotifier, ServerConnectionState>(
  ServerProfileNotifier.new,
);

/// Convenience provider — resolves to `true` when credentials are loaded and valid.
final isConnectedProvider = Provider<bool>((ref) {
  final profileState = ref.watch(serverProfileProvider);
  return profileState.valueOrNull?.isConnected ?? false;
});

/// Provides the active [ServerProfile], or `null` if not connected.
final activeProfileProvider = Provider<ServerProfile?>((ref) {
  final profileState = ref.watch(serverProfileProvider);
  return profileState.valueOrNull?.profile;
});

/// Returns `true` when running in a web browser (i.e. the server URL is
/// auto-detected from the page origin and only the token is entered by the user).
bool get isWebPlatform => kIsWeb;

/// Returns `true` when running on macOS desktop.
bool get isMacOSPlatform =>
    !kIsWeb && defaultTargetPlatform == TargetPlatform.macOS;
