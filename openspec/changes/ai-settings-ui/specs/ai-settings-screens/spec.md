# Spec Delta

## Purpose

Gives org admins, space admins and users screens in the Flutter app to view and change the AI settings they own. The screens show where each value comes from, and stop invalid choices before they are saved.

## ADDED Requirements

### Requirement: Org AI tab

The Admin screen SHALL have an **AI** tab, visible only to org admins. It SHALL have these sections:

- Chat model: provider, searchable model picker, fallback models
- Routing: data collection, ZDR, order, sort, max price
- Reasoning
- Prompt caching
- Web search
- Documents
- Structured output and tools: parallel tool calls
- Images
- Speech-to-text
- Text-to-speech: voice picker, speed slider, format, preview button
- Embeddings
- Decision model
- API keys
- Key health

Only the sections and fields that apply to the selected provider SHALL be shown. Each field SHALL show its source badge (Default, org.toml or Org), a lock toggle for keys that spaces may override, and a "Reset to inherited" action. Saving SHALL first run `validate` and show errors next to their fields. A 409 SHALL show "Settings changed elsewhere" and offer to reload.

#### Scenario: Switch to OpenRouter

- **WHEN** an org admin selects provider OpenRouter
- **THEN** the routing, reasoning, caching, web search, documents and image sections appear, and the model picker lists OpenRouter chat models with context length and price

#### Scenario: Invalid save

- **WHEN** the admin sets both reasoning effort and a token budget and saves
- **THEN** the save is blocked, and the reasoning section shows the error

### Requirement: Secret fields

API key fields SHALL never show a stored value. They SHALL show "Set", "Not set" or "Unreadable", with Replace and Remove actions. Entry SHALL use an obscured field. After a successful save, the field SHALL clear.

#### Scenario: Replace the key

- **WHEN** an admin enters a new OpenRouter key and saves
- **THEN** the field clears, the status shows "Set" with the update time, and key health refreshes

### Requirement: Space AI settings screen

Space admins SHALL reach an **AI settings** screen for the current space from the space menu and from the Admin Spaces tab. It SHALL list the overridable keys, each with:

- its inherited value
- an "Override" switch
- the override control when the switch is on
- a disabled state, with "Locked by your organization", for locked keys

#### Scenario: Locked key

- **WHEN** the org has locked `llm.model`
- **THEN** the model row is disabled and shows the lock message

### Requirement: Voice and AI section in Settings

The user Settings screen SHALL have a **Voice & AI** section with:

- spoken replies (Auto, Always, Never)
- a voice picker limited to the voices the space offers, with a preview
- a speed slider within the model's range
- a "Show reasoning" switch

When the space has no TTS, the voice controls SHALL be hidden and an explanation SHALL be shown.

#### Scenario: No TTS

- **WHEN** the current space has no TTS configured
- **THEN** the voice picker and speed slider are hidden, and the section says voice replies are not configured for this space

### Requirement: Loading, error and empty states

Each screen SHALL show a loading state, an error state with retry, and an empty state when a lookup returns nothing. It SHALL meet the app's accessibility rules: labels for every control, a minimum touch target size, and screen-reader text for source badges and locks. It SHALL use adaptive layouts on phone, tablet and desktop widths.

#### Scenario: Catalog unreachable

- **WHEN** the model lookup fails
- **THEN** the picker shows an error with retry, and still accepts a typed model ID
