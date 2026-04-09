// Service worker for PWA Web Push notifications.
// v__APP_VERSION__ — replaced by build.rs with the version from pubspec.yaml.
//
// Changing this comment (via version bump) causes the browser to treat this
// as a new service worker script and re-install it immediately, rather than
// waiting for the browser's 24-hour byte-diff check.
//
// This file is registered independently of Flutter's generated
// flutter_service_worker.js (which handles offline caching).  Both SWs
// operate at the root scope (/); there are no caching conflicts because they
// handle entirely different event types.
//
// Registration is done from NotificationService.initialize() via
// navigator.serviceWorker.register('/sw.js').

"use strict";

// -- Push event --------------------------------------------------------------
// Fires when the push service delivers a server-sent push message.
self.addEventListener("push", function (event) {
  if (!event.data) {
    console.warn("[sw.js] Push event received with no data — ignoring");
    return;
  }

  let payload;
  try {
    payload = event.data.json();
  } catch (e) {
    // Fallback: treat raw text as the notification body.
    payload = { title: "Assistant", body: event.data.text() };
  }

  const title = payload.title || "Assistant";
  const options = {
    body: payload.body || "",
    icon: "/icons/Icon-192.png",
    badge: "/icons/Icon-192.png",
    data: {
      conversationId: payload.conversationId || null,
      url: payload.conversationId ? "/chat/" + payload.conversationId : "/",
    },
  };

  event.waitUntil(self.registration.showNotification(title, options));
});

// -- Notification click event ------------------------------------------------
// Fires when the user taps a shown notification.
self.addEventListener("notificationclick", function (event) {
  event.notification.close();

  const targetUrl =
    event.notification.data && event.notification.data.url
      ? event.notification.data.url
      : "/";

  event.waitUntil(
    clients
      .matchAll({ type: "window", includeUncontrolled: true })
      .then(function (clientList) {
        // If a window is already open, focus it and navigate.
        for (const client of clientList) {
          if ("focus" in client) {
            client.focus();
            if ("navigate" in client) {
              client.navigate(targetUrl);
            }
            return;
          }
        }
        // Otherwise open a new window.
        if (clients.openWindow) {
          return clients.openWindow(targetUrl);
        }
      }),
  );
});
