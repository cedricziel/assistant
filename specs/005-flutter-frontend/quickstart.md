# Quickstart: Cross-Platform Native App Frontend (005-flutter-frontend)

## Prerequisites

- Flutter SDK (stable channel, 3.x) — `flutter doctor` must pass for web and macOS targets.
- Rust toolchain + `cargo` (for backend changes).
- A running `assistant webui serve` backend (see main project README).

---

## 1. Start the backend

```bash
# From repo root — start the web UI server with a known token
ASSISTANT_WEB_TOKEN=dev-token cargo run -p assistant-cli -- webui serve --listen 127.0.0.1:8080
```

Verify:

```bash
curl http://127.0.0.1:8080/health
# → {"status":"ok"}
```

---

## 2. Run the Flutter app (development)

```bash
cd app

# Install dependencies
flutter pub get

# Run on Chrome (web)
flutter run -d chrome

# Run on macOS desktop
flutter run -d macos
```

On first launch the app shows the connection setup screen. Enter:

- **Server URL**: `http://127.0.0.1:8080`
- **Token**: `dev-token`

Tap **Connect**. The app validates the connection via `GET /health` and stores
the credentials in the platform keychain.

---

## 3. Build distributable artefacts

```bash
cd app

# Web (static files in app/build/web/)
flutter build web

# macOS (.app bundle in app/build/macos/Build/Products/Release/)
flutter build macos
```

---

## 4. Run Flutter tests

```bash
cd app

# Unit + widget tests
flutter test

# Integration tests (requires a running backend at 127.0.0.1:8080)
flutter test integration_test/
```

---

## 5. Run backend tests (new API endpoints)

```bash
# From repo root
cargo test -p assistant-web-ui -- api::personas
cargo test -p assistant-web-ui -- api::traces
cargo test -p assistant-web-ui -- api::logs
```

---

## 6. Validate the SSE streaming

```bash
# Manually verify the streaming endpoint works end-to-end
CONV_ID=$(curl -s -H "Authorization: Bearer dev-token" \
  -H "Content-Type: application/json" \
  -d '{"title":"test"}' \
  http://127.0.0.1:8080/api/conversations | jq -r '.[0].id // .id')

curl -N -H "Authorization: Bearer dev-token" \
     -H "Content-Type: application/json" \
     -d '{"message":"Hello!"}' \
     http://127.0.0.1:8080/api/conversations/$CONV_ID/messages
# Should stream: event:token lines followed by event:done
```

---

## 7. Verify persona switching

```bash
# List personas
curl -H "Authorization: Bearer dev-token" http://127.0.0.1:8080/api/personas

# Switch active persona — replace <PERSONA_ID> with an id from the list above
curl -X POST -H "Authorization: Bearer dev-token" \
     -H "Content-Type: application/json" \
     -d '{"id":"<PERSONA_ID>"}' \
     http://127.0.0.1:8080/api/personas/active
```
