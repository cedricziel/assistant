import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';


/// tests for WebPushApi
void main() {
  final instance = AssistantApi().getWebPushApi();

  group(WebPushApi, () {
    // `POST /api/push/subscribe`
    //
    // Upsert a browser push subscription (endpoint + key material).  Returns `201 Created` on success.
    //
    //Future subscribe(SubscribeRequest subscribeRequest) async
    test('test subscribe', () async {
      // TODO
    });

    // `DELETE /api/push/subscribe`
    //
    // Remove a push subscription by its endpoint URL.  Returns `204 No Content` whether or not the subscription existed.
    //
    //Future unsubscribe(UnsubscribeRequest unsubscribeRequest) async
    test('test unsubscribe', () async {
      // TODO
    });

    // `GET /api/push/vapid-public-key`
    //
    // Returns the server's VAPID public key as a base64url-encoded string. The Flutter PWA uses this as the `applicationServerKey` argument to `PushManager.subscribe()`.
    //
    //Future<VapidKeyResponse> vapidPublicKey() async
    test('test vapidPublicKey', () async {
      // TODO
    });

  });
}
