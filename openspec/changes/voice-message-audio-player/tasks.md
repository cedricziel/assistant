## Tasks

- [ ] Add `audioBytes` (`Uint8List?`) and `audioMimeType` (`String?`) fields to `ChatMessage` in `chat_provider.dart`
- [ ] Update `ChatMessage.copyWith()` to pass through `audioBytes` and `audioMimeType`
- [ ] In `_streamVoiceMessage()`: set `audioBytes` and `audioMimeType` on the initial user `ChatMessage` (line 848)
- [ ] In `_streamVoiceMessage()` `TranscriptEvent` handler: preserve audio fields when updating content via `copyWith`
- [ ] Create `_VoiceMessagePlayer` widget: play/pause button + linear progress bar + duration label, using `audioplayers` `BytesSource`
- [ ] Create `_CollapsibleTranscript` widget: single-line preview with chevron, expandable to full text on tap
- [ ] In `_MessageBubble`: when `message.isUser && message.audioBytes != null`, render `_VoiceMessagePlayer` + `_CollapsibleTranscript` instead of `SelectableText`
- [ ] Handle playback lifecycle: dispose `AudioPlayer` when widget is removed from tree
- [ ] Test: record and send voice message, verify mini player appears with working play/pause
- [ ] Test: tap transcript chevron, verify transcript expands and collapses
- [ ] Test: reload conversation, verify voice message falls back to transcript-only display
- [ ] Test: send voice message while another is playing, verify no audio conflicts
