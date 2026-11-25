# Spec Delta

## Purpose

Ensures PDF attachments reach OpenRouter models as documents, read either natively or by a configurable parser, instead of being silently dropped.

## ADDED Requirements

### Requirement: PDF attachments are sent as file parts

A PDF document attachment SHALL be sent as `{"type": "file", "file": {"filename", "file_data": "data:application/pdf;base64,..."}}` in the user message. When the attachment has no filename, the filename SHALL be `document.pdf`.

#### Scenario: User attaches a PDF

- **WHEN** a user message contains text and a PDF attachment
- **THEN** the request's user content contains a text part and a `file` part carrying the PDF as a data URI

### Requirement: Parser engine

`[llm.openrouter] pdf_engine` SHALL accept `native`, `cloudflare-ai` or `mistral-ocr`. When it is set, requests that contain a PDF SHALL include `plugins: [{"id": "file-parser", "pdf": {"engine": <value>}}]`. When it is unset, the provider SHALL send no parser plugin for models with native file input, and SHALL use `cloudflare-ai` for all other models. A request with web search enabled SHALL carry both plugins.

#### Scenario: Text-only model with a PDF

- **WHEN** the model has no native file input, `pdf_engine` is unset, and a PDF is attached
- **THEN** the request includes the `file-parser` plugin with engine `cloudflare-ai`

#### Scenario: OCR for scanned documents

- **WHEN** `pdf_engine = "mistral-ocr"` is set and a PDF is attached
- **THEN** the request includes the `file-parser` plugin with engine `mistral-ocr`
