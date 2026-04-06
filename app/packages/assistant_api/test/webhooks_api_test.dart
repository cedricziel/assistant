import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';


/// tests for WebhooksApi
void main() {
  final instance = AssistantApi().getWebhooksApi();

  group(WebhooksApi, () {
    // `POST /api/webhooks` — create a new webhook.
    //
    //Future<WebhookResponse> createWebhook(CreateWebhookRequest createWebhookRequest) async
    test('test createWebhook', () async {
      // TODO
    });

    // `DELETE /api/webhooks/{id}` — delete a webhook.
    //
    //Future deleteWebhook(String id) async
    test('test deleteWebhook', () async {
      // TODO
    });

    // `GET /api/webhooks/{id}` — get a webhook by ID.
    //
    //Future<WebhookResponse> getWebhook(String id) async
    test('test getWebhook', () async {
      // TODO
    });

    // `GET /api/webhooks` — list all webhooks.
    //
    //Future<BuiltList<WebhookResponse>> listWebhooks() async
    test('test listWebhooks', () async {
      // TODO
    });

    // `POST /api/webhooks/{id}/rotate-secret` — regenerate a webhook's signing secret.
    //
    //Future<RotateSecretResponse> rotateSecret(String id) async
    test('test rotateSecret', () async {
      // TODO
    });

    // `POST /api/webhooks/{id}/toggle` — toggle a webhook's active state.
    //
    //Future<WebhookResponse> toggleWebhook(String id) async
    test('test toggleWebhook', () async {
      // TODO
    });

    // `PATCH /api/webhooks/{id}` — update a webhook.
    //
    //Future<WebhookResponse> updateWebhook(String id, UpdateWebhookRequest updateWebhookRequest) async
    test('test updateWebhook', () async {
      // TODO
    });

    // `POST /api/webhooks/{id}/verify` — send a signed test payload to the webhook URL.
    //
    // Always returns HTTP 200; the `success` field in the body indicates whether the remote endpoint responded with a 2xx status.
    //
    //Future<VerifyWebhookResponse> verifyWebhook(String id) async
    test('test verifyWebhook', () async {
      // TODO
    });

  });
}
