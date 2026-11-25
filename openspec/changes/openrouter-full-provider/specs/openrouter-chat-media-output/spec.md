# Spec Delta

## Purpose

Lets image-capable and audio-capable chat models answer with images, or with spoken audio in voice conversations. The media is delivered alongside the text answer.

## ADDED Requirements

### Requirement: Media on chat responses

A chat response SHALL be able to carry generated media: images and at most one audio clip. Each item SHALL carry its MIME type and bytes. The orchestrator SHALL persist each item as an attachment of the assistant message and deliver it per `image-outbound` and `channel-audio-outbound`.

#### Scenario: Answer with an image

- **WHEN** a final answer carries one PNG image
- **THEN** the image is stored as an attachment of that assistant message and appears in the reply on every interface

### Requirement: Chat image output

When `[llm.openrouter] image_output = true` is set and the catalog lists image output for the model, chat requests SHALL include `modalities: ["image", "text"]`, plus `image_config` from `[llm.openrouter.image_config]`. Each `message.images[].image_url.url` SHALL be decoded into image media: data URIs are decoded directly, and HTTPS URLs are downloaded at once. Images found in streamed deltas or in the final message SHALL both be collected, without duplicates. Setting `image_output = true` for a model without image output SHALL make every chat call fail with an error naming the model, and the error SHALL be logged at startup.

#### Scenario: Gemini image model

- **WHEN** `image_output = true` is set for a model with image output, and the reply contains one base64 PNG in `message.images`
- **THEN** the final answer carries that image as `image/png` media

### Requirement: Voice turns

A turn whose user message came from a voice message SHALL be treated as a voice turn. Interfaces mark it by attaching the original recording (see `openrouter-chat-audio-input`).

#### Scenario: Signal voice note

- **WHEN** a Signal voice note is transcribed and passed to the orchestrator
- **THEN** that turn carries the recording and is treated as a voice turn

### Requirement: Native audio replies

`[llm.openrouter.audio_output]` SHALL accept `enabled` (default `false`), `voice` (default `alloy`) and `pcm_sample_rate` (default 24000). When it is enabled, the turn is a voice turn and the catalog lists audio output for the model, chat requests in that turn SHALL stream with `modalities: ["text", "audio"]` and `audio: {"voice", "format": "pcm16"}`. Base64 `delta.audio.data` chunks SHALL be concatenated in order. When `delta.audio.transcript` text arrives and there is no `delta.content`, the transcript SHALL become the answer text. The PCM SHALL be read at `pcm_sample_rate` and transcoded to the TTS output format (`[tts] format`, default `mp3`) per `openrouter-speech`. The result SHALL be attached as the reply's audio clip.

#### Scenario: Spoken reply to a voice note

- **WHEN** audio output is enabled, a user sends a voice note, and the model streams audio with a transcript
- **THEN** the reply carries the transcript as text and one audio clip in the configured format

### Requirement: Audio reply fallback

When the upstream rejects audio output for a request, for example in combination with tools, the provider SHALL repeat that call once without audio. It SHALL not request audio again for the rest of the provider instance's life, and SHALL log the reason once. When a voice turn's final answer has no audio clip and `[tts]` is configured, the system SHALL synthesize the answer with the TTS provider and attach that clip instead.

#### Scenario: Audio rejected with tools

- **WHEN** a voice-turn request with tools and audio output returns 400 for the audio modality
- **THEN** the call is repeated without audio, the answer is synthesized through `[tts]`, and later turns do not request audio again

#### Scenario: No TTS configured

- **WHEN** native audio is unavailable and `[tts]` is absent
- **THEN** the reply is text only, and no error is shown
