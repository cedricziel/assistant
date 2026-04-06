import 'package:assistant_api/assistant_api.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../api/api_client.dart';
import '../connection/connection_provider.dart';

/// State for the webhooks list screen.
class WebhooksState {
  const WebhooksState({
    this.webhooks = const [],
    this.isLoading = false,
    this.error,
  });

  final List<WebhookResponse> webhooks;
  final bool isLoading;
  final String? error;
}

/// Notifier for the webhooks list.
class WebhooksNotifier extends AsyncNotifier<WebhooksState> {
  @override
  Future<WebhooksState> build() async {
    return _fetchWebhooks();
  }

  ApiClient? get _api {
    final profile = ref.read(activeProfileProvider);
    if (profile == null) return null;
    return ApiClient(baseUrl: profile.baseUrl, token: profile.token);
  }

  Future<WebhooksState> _fetchWebhooks() async {
    final api = _api;
    if (api == null) return const WebhooksState();

    try {
      final response = await api.webhooks.listWebhooks();
      final webhooks = response.data!.toList();
      return WebhooksState(webhooks: webhooks);
    } catch (e) {
      return WebhooksState(error: e.toString());
    }
  }

  Future<void> refresh() async {
    state = const AsyncLoading();
    state = AsyncData(await _fetchWebhooks());
  }
}

/// Provider for [WebhooksNotifier].
final webhooksProvider =
    AsyncNotifierProvider.autoDispose<WebhooksNotifier, WebhooksState>(
  WebhooksNotifier.new,
);

// ---------------------------------------------------------------------------
// Webhook detail

/// State for a single webhook detail screen.
class WebhookDetailState {
  const WebhookDetailState({
    this.webhook,
    this.isLoading = false,
    this.error,
    this.actionMessage,
  });

  final WebhookResponse? webhook;
  final bool isLoading;
  final String? error;
  final String? actionMessage;
}

/// Notifier for a single webhook detail.
class WebhookDetailNotifier extends AsyncNotifier<WebhookDetailState> {
  WebhookDetailNotifier(this._webhookId);

  final String _webhookId;

  @override
  Future<WebhookDetailState> build() async {
    return _fetchDetail(_webhookId);
  }

  ApiClient? get _api {
    final profile = ref.read(activeProfileProvider);
    if (profile == null) return null;
    return ApiClient(baseUrl: profile.baseUrl, token: profile.token);
  }

  Future<WebhookDetailState> _fetchDetail(String webhookId) async {
    final api = _api;
    if (api == null) return const WebhookDetailState();

    try {
      final response = await api.webhooks.getWebhook(id: webhookId);
      return WebhookDetailState(webhook: response.data!);
    } catch (e) {
      return WebhookDetailState(error: e.toString());
    }
  }

  Future<void> refresh() async {
    state = const AsyncLoading();
    state = AsyncData(await _fetchDetail(_webhookId));
  }

  Future<void> verify() async {
    final api = _api;
    if (api == null) return;
    final current = state.value ?? const WebhookDetailState();
    state = AsyncData(current.copyWith(isLoading: true));
    try {
      await api.webhooks.verifyWebhook(id: _webhookId);
      final updated = await _fetchDetail(_webhookId);
      state = AsyncData(
        updated.copyWith(actionMessage: 'Verification request sent'),
      );
    } catch (e) {
      state = AsyncData(
        current.copyWith(isLoading: false, error: e.toString()),
      );
    }
  }

  Future<void> rotateSecret() async {
    final api = _api;
    if (api == null) return;
    final current = state.value ?? const WebhookDetailState();
    state = AsyncData(current.copyWith(isLoading: true));
    try {
      await api.webhooks.rotateSecret(id: _webhookId);
      final updated = await _fetchDetail(_webhookId);
      state = AsyncData(
        updated.copyWith(actionMessage: 'Secret rotated'),
      );
    } catch (e) {
      state = AsyncData(
        current.copyWith(isLoading: false, error: e.toString()),
      );
    }
  }
}

extension _WebhookDetailStateCopyWith on WebhookDetailState {
  WebhookDetailState copyWith({
    WebhookResponse? webhook,
    bool? isLoading,
    String? error,
    String? actionMessage,
  }) {
    return WebhookDetailState(
      webhook: webhook ?? this.webhook,
      isLoading: isLoading ?? this.isLoading,
      error: error ?? this.error,
      actionMessage: actionMessage ?? this.actionMessage,
    );
  }
}

/// Family provider for [WebhookDetailNotifier].
final webhookDetailProvider = AsyncNotifierProvider.autoDispose
    .family<WebhookDetailNotifier, WebhookDetailState, String>(
  (arg) => WebhookDetailNotifier(arg),
);
