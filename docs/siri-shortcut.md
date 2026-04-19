# Siri & Action Button Integration (iOS)

Ask your assistant questions via Siri or the iPhone Action Button and hear spoken responses.

## Prerequisites

- iOS 16.0 or later
- The assistant server running with `--auth-token` configured
- The iOS app connected to your server (active context with credentials)

## How it works

When you trigger the shortcut, Siri prompts "What would you like to ask?", sends your question to `POST /api/quick-message` on your server, and speaks the answer back.

```
Action Button / Siri
  → "What would you like to ask?"
  → You speak your question
  → POST /api/quick-message (Bearer auth, 25s timeout)
  → Siri reads the answer aloud
```

Each question creates a new conversation with an auto-generated title.

## Setup

### 1. Connect the app to your server

Open the iOS app and add your server connection (URL + auth token). The app automatically syncs credentials to the iOS Keychain so Siri can access them.

### 2. Find the shortcut

The shortcut registers automatically. You can find it in:

- **Shortcuts app** — search for "Ask Assistant"
- **Siri** — say "Ask Assistant a question" or "Ask Assistant something"

### 3. Assign to Action Button (iPhone 15 Pro+)

1. Open **Settings → Action Button**
2. Swipe to **Shortcut**
3. Tap **Choose a Shortcut**
4. Search for **"Ask Assistant"** and select it

Now pressing the Action Button triggers Siri to ask for your question.

### 4. Alternative: add to Lock Screen

1. Long-press the Lock Screen → **Customize**
2. Tap a Lock Screen button slot
3. Choose **Shortcut → Ask Assistant**

## Siri phrases

These phrases are registered automatically:

- "Ask _Assistant_ a question"
- "Ask _Assistant_ something"

Where _Assistant_ is your app's display name.

## Error handling

| Situation                 | Siri says                                                               |
| ------------------------- | ----------------------------------------------------------------------- |
| No credentials configured | "Please open the app and connect to your assistant server first."       |
| Server takes > 25 seconds | "I'm still working on that. Check the app for the full answer."         |
| Network unreachable       | "I couldn't reach your assistant server. Please check your connection." |

## API endpoint

The shortcut uses the synchronous quick-message endpoint:

```
POST /api/quick-message
Authorization: Bearer <token>
Content-Type: application/json

{"message": "What is the weather today?"}
```

Response (201):

```json
{
  "conversation_id": "uuid",
  "message_id": "uuid",
  "answer": "The assistant's response text..."
}
```

This endpoint creates a new conversation, sends the message to the active persona's orchestrator, waits for the complete response, and returns it as JSON.
