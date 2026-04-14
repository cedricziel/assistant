## 1. TTS Infrastructure (assistant-transcription + assistant-core)

- [ ] 1.1 Add `TtsRequest` and `TtsResult` types to `crates/transcription/src/provider.rs`
- [ ] 1.2 Add `TtsProvider` trait to `crates/transcription/src/provider.rs` with `name()` and `synthesize()` methods
- [ ] 1.3 Add `TtsConfig` struct to `crates/core/src/types.rs` (fields: provider, model, api_key, base_url, voice, language)
- [ ] 1.4 Add `TtsProviderKind` enum to `crates/core/src/types.rs` (variants: OpenAI, Deepgram)
- [ ] 1.5 Add `tts: Option<TtsConfig>` field to `AssistantConfig` in `crates/core/src/types.rs`
- [ ] 1.6 Implement `OpenAITtsProvider` in `crates/transcription/src/openai_tts.rs` (POST /v1/audio/speech, returns mp3)
- [ ] 1.7 Implement `DeepgramTtsProvider` in `crates/transcription/src/deepgram_tts.rs` (POST /v1/speak)
- [ ] 1.8 Add `build_tts_provider(config: &TtsConfig) -> Result<Arc<dyn TtsProvider>>` to `crates/transcription/src/lib.rs`
- [ ] 1.9 Export new types from `crates/transcription/src/lib.rs`
- [ ] 1.10 Write unit tests for `TtsRequest`/`TtsResult` types and provider name methods

## 2. AudioStore and Web UI State

- [ ] 2.1 Create `AudioStore` struct in `crates/web-ui/src/audio_store.rs` (in-memory `HashMap<Uuid, (Vec<u8>, Instant)>` with 1-hour TTL)
- [ ] 2.2 Add `insert()`, `get()`, and `sweep()` methods to `AudioStore`
- [ ] 2.3 Add `tts_provider: Option<Arc<dyn TtsProvider>>` and `audio_store: Arc<AudioStore>` to `ApiState`
- [ ] 2.4 Wire transcription provider into web-ui `main.rs` startup (read `config.transcription`, call `build_provider()`)
- [ ] 2.5 Wire TTS provider into web-ui `main.rs` startup (read `config.tts`, call `build_tts_provider()`)
- [ ] 2.6 Spawn background TTL sweep task in `main.rs` (every 10 minutes calls `audio_store.sweep()`)

## 3. Server Endpoints (Rust)

- [ ] 3.1 Add `GET /api/capabilities` handler returning `{"voice_send": bool, "voice_receive": bool}` based on `ApiState`
- [ ] 3.2 Add `POST /api/conversations/{id}/voice` handler: parse multipart, validate MIME type, enforce 25 MB limit, transcribe, run through orchestrator, return SSE stream
- [ ] 3.3 Add `GET /api/messages/{msg_id}/audio` handler: fetch message from DB, validate it is an assistant message, synthesize via TtsProvider, return `audio/mpeg`
- [ ] 3.4 Add `GET /api/audio/{audio_id}` handler: look up in `AudioStore`, return `audio/mpeg` or 404
- [ ] 3.5 Register new routes in `api_router()` in `crates/web-ui/src/api/mod.rs`
- [ ] 3.6 Add utoipa path annotations to all new handlers
- [ ] 3.7 Update `openapi.json` snapshot (`make dump-openapi`)
- [ ] 3.8 Write unit tests for MIME type validation and 25 MB size check

## 4. `voice_response` Tool

- [ ] 4.1 Create `crates/tool-executor/src/builtins/voice_response.rs` with `VoiceResponseHandler` struct holding `Arc<dyn TtsProvider>` and `Arc<AudioStore>`
- [ ] 4.2 Implement `ToolHandler` for `VoiceResponseHandler`: name `"voice-response"`, params `{text: string, voice?: string}`, run: synthesize → store → return `{audio_id}`
- [ ] 4.3 Export `VoiceResponseHandler` from `crates/tool-executor/src/builtins/mod.rs`
- [ ] 4.4 Register `VoiceResponseHandler` in `ToolExecutor::register_builtins()` only when TTS is configured
- [ ] 4.5 Emit `audio_ready` SSE event when a `voice_response` tool result is detected in the orchestrator SSE handler in `crates/web-ui/src/api/mod.rs`
- [ ] 4.6 Write unit tests for `VoiceResponseHandler::run()` with a mock TtsProvider

## 5. Flutter Packages and API Client

- [ ] 5.1 Add `record: ^5.0.0` and `audioplayers: ^6.0.0` to `app/pubspec.yaml` and run `flutter pub get`
- [ ] 5.2 Add `NSMicrophoneUsageDescription` to `app/macos/Runner/Info.plist`
- [ ] 5.3 Add `capabilities()` method to the Flutter API client (GET /api/capabilities, returns `ServerCapabilities` model)
- [ ] 5.4 Add `sendVoiceMessage(conversationId, audioBytes, mimeType)` method to the API client
- [ ] 5.5 Add `fetchMessageAudio(messageId)` method returning `Uint8List` (GET /api/messages/{id}/audio)
- [ ] 5.6 Add `fetchAudio(audioId)` method returning `Uint8List` (GET /api/audio/{id})
- [ ] 5.7 Create `ServerCapabilities` model class with `voiceSend` and `voiceReceive` fields
- [ ] 5.8 Add `capabilitiesProvider` Riverpod provider that fetches and caches `ServerCapabilities`

## 6. Flutter UI — Voice Send

- [ ] 6.1 Create `VoiceRecorder` widget in `app/lib/features/chat/voice_recorder.dart` (manages record state, timer, error handling)
- [ ] 6.2 Add mic button to `_InputRow` in `chat_screen.dart` — shown when `capabilities.voiceSend == true`
- [ ] 6.3 Wire mic button tap to toggle `VoiceRecorder` recording state
- [ ] 6.4 Show recording indicator (pulsing red dot) and countdown timer while recording
- [ ] 6.5 On recording stop, call `chatProvider.notifier.sendVoiceMessage(bytes, mimeType)`
- [ ] 6.6 Add `sendVoiceMessage()` to `ChatNotifier` in `chat_provider.dart` (upload → SSE stream same as text send)
- [ ] 6.7 Show error snackbar for permission denied and server 503 responses
- [ ] 6.8 Auto-stop recording at 2 minutes

## 7. Flutter UI — Voice Receive

- [ ] 7.1 Create `AudioPlayerWidget` in `app/lib/features/chat/audio_player_widget.dart` (play/stop button, fetches audio on first tap, plays via `audioplayers`)
- [ ] 7.2 Add play button to `_MessageBubble` for assistant messages when `capabilities.voiceReceive == true`
- [ ] 7.3 Wire play button to `AudioPlayerWidget` passing the message ID
- [ ] 7.4 Handle `audio_ready` SSE event in `ChatNotifier`: extract `audio_id`, fetch audio, auto-play
- [ ] 7.5 Ensure play button shows ■ (stop) while audio is playing and reverts to ▶ on completion or stop

## 8. Integration and Polish

- [ ] 8.1 Regenerate Flutter API client (`make generate-flutter-client`) after OpenAPI spec update
- [ ] 8.2 Run `flutter analyze` and fix any issues
- [ ] 8.3 Run `make lint` and `make format` on Rust code
- [ ] 8.4 Run `make test` and verify no regressions
- [ ] 8.5 Manual smoke test: send voice → see transcript as user message → assistant responds → tap play → hear audio
- [ ] 8.6 Manual smoke test: assistant uses `voice_response` tool → audio auto-plays
- [ ] 8.7 Manual smoke test with TTS not configured: mic button hidden, play button hidden
