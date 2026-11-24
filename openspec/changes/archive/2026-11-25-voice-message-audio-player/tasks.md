## Tasks

- [x] Add `audioBytes` (`Uint8List?`) and `audioMimeType` (`String?`) fields to `ChatMessage` in `chat_provider.dart`
- [x] Update `ChatMessage.copyWith()` to pass through `audioBytes` and `audioMimeType`
- [x] In `_streamVoiceMessage()`: set `audioBytes` and `audioMimeType` on the initial user `ChatMessage` (line 848)
- [x] In `_streamVoiceMessage()` `TranscriptEvent` handler: preserve audio fields when updating content via `copyWith`
- [x] Create `_VoiceMessagePlayer` widget: play/pause button + linear progress bar + duration label, using `audioplayers` `BytesSource`
- [x] Create `_CollapsibleTranscript` widget: single-line preview with chevron, expandable to full text on tap
- [x] In `_MessageBubble`: when `message.isUser && message.audioBytes != null`, render `_VoiceMessagePlayer` + `_CollapsibleTranscript` instead of `SelectableText`
- [x] Handle playback lifecycle: dispose `AudioPlayer` when widget is removed from tree
- [ ] Test: record and send voice message, verify mini player appears with working play/pause
- [ ] Test: tap transcript chevron, verify transcript expands and collapses
- [ ] Test: reload conversation, verify voice message falls back to transcript-only display
- [ ] Test: send voice message while another is playing, verify no audio conflicts
