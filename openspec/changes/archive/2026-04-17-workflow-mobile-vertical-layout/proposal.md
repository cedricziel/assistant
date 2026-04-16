## Why

The workflow editor uses a free-form canvas (3000×3000 virtual space with draggable nodes and drawn edges) that is impractical on mobile — precise touch-drag for positioning nodes and drawing edges is frustrating on small screens. Users on phones and tablets cannot effectively compose or edit workflows today.

## Layout Comparison

```
DESKTOP (canvas, ≥ 600 px)          MOBILE (card list, < 600 px)
─────────────────────────────        ──────────────────────────────
┌─────────────────────────────┐      ┌────────────────────────────┐
│  Workflow Editor Canvas      │      │  < Workflows   [Save]      │
│                              │      ├────────────────────────────┤
│                              │      │  Name: My Workflow          │
│    ┌──────────────┐          │      ├────────────────────────────┤
│    │  ⚡ Trigger  │          │      │                            │
│    │  Manual      │          │      │ ╔══════════════════════╗  │
│    └──────┬───────┘          │      │ ║ ⚡ Trigger           ║  │
│           │                  │      │ ║  Manual              ║  │
│    ┌──────▼───────┐          │      │ ║                   ⠿ ║  │
│    │  ⚡ Action   │          │      │ ╚══════════════════════╝  │
│    │  HTTP Request│          │      │           │                │
│    └──────┬───────┘          │      │           ▼                │
│           │                  │      │ ╔══════════════════════╗  │
│    ┌──────▼───────┐          │      │ ║ ⚡ Action            ║  │
│    │  ✦ Condition │          │      │ ║  HTTP Request        ║  │
│    │  Key Check   │          │      │ ║                   ⠿ ║  │
│    └──┬───────┬───┘          │      │ ╚══════════════════════╝  │
│    true│   false│            │      │           │                │
│    ┌───▼──┐ ┌───▼──┐        │      │           ▼                │
│    │ Act A│ │ Act B│        │      │ ╔══════════════════════╗  │
│    └──────┘ └──────┘        │      │ ║ ✦ Condition          ║  │
│                              │      │ ║  Key Check           ║  │
│  ← pan/scroll canvas →      │      │ ║                   ⠿ ║  │
└─────────────────────────────┘      │ ╚══════════════════════╝  │
                                      │                            │
 Nodes: freely positioned             │    ┌──── [+ Add step] ─┐  │
 Edges: drawn by drag                 │    └───────────────────┘  │
 Mobile: ✗ unusable                   └────────────────────────────┘

                                       Nodes: ordered cards
                                       Edges: inferred top→bottom
                                       Mobile: ✓ touch-friendly
```

## What Changes

- Introduce a **mobile vertical layout** for the workflow editor that renders when the device width is below a breakpoint (~600 px).
- In mobile view, nodes (trigger, actions, conditions) are presented as scrollable vertical cards in a fixed linear list — no canvas, no freehand positioning.
- Edges are implicit: each card connects to the card below it; branching (condition true/false) is represented inline.
- Users add nodes via a bottom-sheet palette (already exists), reorder via drag-handle, and delete via swipe or icon.
- The desktop canvas view remains unchanged.
- No API or data-model changes — the same graph JSON is built from the ordered card list.

## Capabilities

### New Capabilities

- `workflow-mobile-editor`: Mobile-optimised, vertically-stacked card editor for workflows, rendered when `MediaQuery` width < 600 px. Covers layout, node card design, add/remove/reorder interactions, and edge-inference from card order.

### Modified Capabilities

- none

## Impact

- **Flutter app only** — `app/lib/features/workflows/workflow_editor_screen.dart`.
- No Rust / backend changes required.
- No new dependencies expected (uses existing Flutter Material widgets + Riverpod).
- The existing `_EdgePainter`, `EditorNode`, and `EditorEdge` model classes are reused; only the rendering layer is branched by screen width.
