# AGENTS.md — Flutter app

### State Management

Riverpod 2.x (`flutter_riverpod`). Providers live in `*_provider.dart` files alongside their screens. All async providers use `AsyncNotifier` / `AutoDisposeAsyncNotifier`.

### CORS

The Rust web-ui server emits `Access-Control-Allow-Origin` headers for Flutter web. Configure the allowed origin with:

```sh
assistant webui serve --cors-origin http://localhost:4040
# or via env var:
ASSISTANT_WEB_CORS_ORIGIN=http://localhost:4040 assistant webui serve
```

### CI

GitHub Actions runs `flutter analyze --fatal-infos` and `flutter test` on every push/PR that touches `app/**` (see `.github/workflows/flutter.yml`). `flutter analyze` must report zero issues.
