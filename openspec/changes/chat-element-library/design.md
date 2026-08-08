# Design — chat-element-library

## Context

assistant-ui Elements is a React + Tailwind registry distributed shadcn-style. There is no mechanical port path to Flutter: no shared runtime, no Tailwind, no registry convention. What transfers is the **vocabulary** (which pieces an assistant conversation is made of) and the **gallery model** (every element demoable in isolation, in every state).

This change adopts both for the display half of the vocabulary — shell, thinking, streaming, display — **and the composer**. It excludes everything that requires a client→server return channel, and everything with no data behind it today. `elements.md` carries the per-element verdict.

## How assistant-ui composes, and the Flutter translation

assistant-ui's primitives are React compound components: `<ThreadPrimitive.Root>` provides context, descendants consume it. Flutter offers three translations, and the choice determines whether this ends up a design system or a folder.

|       | Approach                                                                                                                                                                                                  | Verdict                             |
| ----- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------- |
| **A** | **Constructor slots** — `TimelineEntryShell(entry: …, body: …)`. Explicit, no magic, golden-testable with no wrapper, deps visible in the signature. Verbose at depth.                                    | **Default for everything.**         |
| **B** | **Inherited scope** — `ThreadDensityScope.of(context)`. Closest to React context, ergonomic at depth. Implicit deps; goldens need a wrapper.                                                              | **Density only.**                   |
| **C** | **Riverpod inside elements** — `ref.watch(chatProvider)` in the element. Ergonomic, but couples elements to app state, breaks the gallery, and forces `ProviderScope` + overrides into every widget test. | **Banned by the package boundary.** |

**Decision: A everywhere, B for density alone, C nowhere.**

The evidence that A is right is that it already works: `StreamingTimelineEntry(message, density, entryState)` is a pure constructor-slot element with zero context reads. The extraction formalises what the code already does rather than imposing a new pattern.

Density earns B because every element at every nesting level needs it and nothing else does — threading one value through five levels is the exact case an `InheritedWidget` exists for. `EntryState` stays a constructor param: it is per-entry, not ambient.

## The package boundary is the mechanism

```
app/packages/assistant_ui/pubspec.yaml
  dependencies:
    flutter: sdk        ← and effectively nothing else
    # NOT flutter_riverpod
    # NOT assistant_api
```

Discipline alone has already failed once — `TurnProgressCard` reaches for `ref` three times. A package that cannot resolve `package:flutter_riverpod` turns "elements must be pure" from a review comment into a compile error. This is the whole reason to pay the monorepo-package ceremony rather than using `lib/features/chat/elements/`.

The rule is not merely "no state management" — it is **no I/O**. The package renders and holds local widget state; anything that touches a device, a file system or a socket stays in the app. That draws a clean line through three existing dependencies:

| Dependency                    | Side        | Why                                                                          |
| ----------------------------- | ----------- | ---------------------------------------------------------------------------- |
| `flutter_smooth_markdown`     | **package** | Pure rendering. `MessageBody`/`ThinkingBody` need it for streaming markdown. |
| `record`                      | app         | Microphone I/O.                                                              |
| `file_picker`, `desktop_drop` | app         | File-system and platform-channel I/O.                                        |

**The allowlist is closed**: `flutter` and `flutter_smooth_markdown`, nothing else. `proposal.md`, this document and the spec all state that same list; the boundary test in task 1.1 enforces it in both directions — forbidden entries in `pubspec.yaml`, and forbidden `import` statements (`dart:io`, `record`, `file_picker`, `desktop_drop`) anywhere in package source. A manifest check alone would not catch a transitively-available I/O import.

## Reasoning: what the data actually supports

`ReasoningPanel` was initially specified as a step timeline with per-step elapsed time. It cannot be, and the reason is worth recording rather than quietly dropping.

```
runtime          OrchestratorEvent::Thinking(String)   ← undifferentiated tokens
client model     ChatMessage.thinkingContent: String?  ← flat
                 ChatMessage.thinkingTokenStream       ← Stream<String>
```

There is no step delimiter anywhere in the chain and no per-step clock. Deriving boundaries from token arrival times would invent structure the model never expressed — the panel would show step divisions that correspond to network chunking, not to reasoning.

**Decision: ship the collapsible section, defer the timeline.** `ReasoningPanel` takes `thinkingContent`/`thinkingTokenStream` and a single `elapsed` for the whole reasoning block — all of which exist today — and renders a collapsible section with total elapsed time. This keeps task 2.1's "move the models unchanged" intact.

The per-step timeline is **P5** in `elements.md`. Unblocking it means either delimited steps on the wire or a package-owned `ReasoningStep` model with defined boundary and timing semantics — a decision about the protocol, not about a widget, and therefore not one to make inside a refactor.

## Composer

`_InputRow` is the best-shaped widget in `features/chat` already: 13 constructor parameters, every interaction a callback (`onSend`, `onStop`, `onVoiceRecorded`, `onPickImage`, `onRemoveAttachment`, `onPasteImage`), zero `ref` reads. It becomes `Composer` essentially unchanged. `SlashCommandMenu` follows from `command_autocomplete.dart` with the command list supplied by the app.

Voice is the one that has to be cut in half. `VoiceRecorderButton` is a `ConsumerStatefulWidget` that imports `record` — it both renders the mic affordance and drives the recorder. The package takes `ComposerVoiceButton`: mic icon, recording countdown, stop affordance, driven by an `isRecording` parameter and emitting `onStart`/`onStop`. The app keeps the recorder and feeds it. This is the general shape for any element that currently mixes presentation with a plugin.

Two composer elements are declined for want of data rather than want of design:

- **Mentions** — `grep -rn "mention"` over `app/lib` and `crates/web-ui` returns nothing. There is no mentionable-entity concept to render.
- **Context / token ring** — token counts exist only as analytics aggregates (`token_usage_over_time`, `total_tokens_in/out`). Nothing reports live per-conversation usage, so the ring would have no value to display. It is listed as deferred behind P1 in `elements.md`, not declined outright.

## The element index

`elements.md` records a verdict for all 59 assistant-ui elements: Adopt, Adopt (have), Defer behind a named prerequisite, or Decline with a reason. It moves to `app/packages/assistant_ui/ELEMENTS.md` on PR 1 and is maintained there.

It exists because the failure mode of adopting a catalogue is drift in both directions — quietly building something previously declined, or forgetting that a Defer became unblocked. Recording the _prerequisite_ rather than a bare "no" is what makes the second recoverable: the seven P1 elements become nearly free the day `ToolOutput.data` reaches the wire, and the index is what says so.

## Shell / body split

Today one 669-line `State` class owns density, expand, `_userPinned`, the auto-collapse timer, reduced-motion handling, `EntryState`, **and** the rendering for all five entry kinds. `ChatTimelineSection` (302 LOC) independently reimplements the collapse half. Adding an element means growing one of those two files.

```
TimelineEntryShell  (stateful — the ONLY stateful element)
  density · expanded · userPinned · autoCollapseTimer · stale · header row
        │
        └── body: Widget   ← slot
                 │
    MessageBody · ThinkingBody · ToolCallBody · SubagentBody · CommandBody
    stateless · one golden test each · no timers, no MediaQuery
```

One state machine tested once; N bodies tested as pure functions of their input. This is also the seam that later admits tool-derived renderers (terminal block, code diff) and, later still, agent-composed blocks — both slot into `body` without touching the shell.

## Moving the models

`ChatMessage`, `ChatAttachment`, `ToolCallRecord`, `MessageStatus`, `ToolCallStatus` and `TimelineEntryType` are pure data classes importing only `dart:async` and `dart:typed_data`. They move into the package as its public surface; `chat_provider.dart` imports them back.

**Decision: move `ChatMessage` as-is. Do not convert it to a sealed hierarchy in this change.**

It is a union-by-nullable-fields — `thinkingContent`, `subagentId`, `subagentTask`, `commandName` are each null unless `timelineType` matches. A sealed hierarchy is the better model, but:

- `ChatNotifier` mutates `content`, `isStreaming`, `isStale`, `status` and `subagentContent` **in place** during streaming, deliberately, for rendering throughput. Sealing implies immutability, which implies rewriting the streaming path.
- The blast radius is the whole 2306-line provider, which this change otherwise does not touch.
- The shell/body split delivers most of the benefit at the widget layer regardless: each body reads only the fields for its own kind, so the nullable-union stops leaking into rendering even while it persists in the model.

Sealing is a legitimate follow-up once the widget layer is stable. Recording it here so it is a decision, not an oversight.

## Widgetbook

`app/widgetbook/` is a separate Flutter app depending on `assistant_ui` only. Without it this change is code motion; with it, the twelve states of a timeline entry (`active`/`complete`/`stale` × `compact`/`normal`/`expanded`) become directly inspectable instead of reachable only by luck against a live stream.

It is excluded from `flutter build web` for the embedded SPA — it is a development surface, not a shipped one.

## Risks

- **Test churn.** Existing chat widget tests import from `features/chat`. They move with the widgets in the same task so coverage never regresses (per the project's tasks rules).
- **Golden-test flake across platforms.** Goldens are notoriously host-sensitive. Mitigation: goldens run on CI's Linux runner only, consistent with the existing Playwright tolerance approach in `.claude/skills/e2e-testing`.
- **Baseline movement.** `ReasoningPanel` and the scroll-anchor pill are genuinely new UI. Playwright baselines are updated as an explicit task, not as a surprise diff.
