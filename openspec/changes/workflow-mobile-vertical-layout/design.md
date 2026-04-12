## Context

The workflow editor (`workflow_editor_screen.dart`) renders a free-form 3000×3000 canvas with draggable `EditorNode` widgets and a `CustomPainter` drawing bezier edges. This model suits desktop/trackpad users but is unusable on mobile: node-drag positioning and edge-draw gestures conflict with scroll events and require sub-centimetre precision.

The mobile alternative must:

- Reuse the existing `EditorNode` / `EditorEdge` data model unchanged.
- Produce identical `_buildGraph()` JSON so the backend needs no changes.
- Coexist with the desktop canvas in the same screen file.

## Goals / Non-Goals

**Goals:**

- Render a scrollable vertical list of node cards when `MediaQuery.of(context).size.width < 600`.
- Allow add, delete, and reorder of nodes entirely via tap/long-press — no drag-on-canvas.
- Infer edges from card order (top → bottom) rather than requiring explicit edge-drawing.
- Support branching condition nodes (true/false child slots shown inline beneath the condition card).
- Parity with desktop: same node types, same save flow, same validation.

**Non-Goals:**

- Replacing the canvas on desktop (desktop view unchanged).
- Supporting arbitrary DAGs with multiple parents on mobile (linear chains + single condition branching only).
- Persisting mobile vs. desktop layout preference — layout is purely derived from screen width at runtime.
- Backward compatibility shims, feature flags, or gradual rollout — the change ships in one PR; old code paths are deleted, not wrapped.

## Decisions

### D1 — `LayoutBuilder` / `MediaQuery` breakpoint at 600 px logical pixels

`WorkflowEditorScreen.build()` wraps its body in `LayoutBuilder`. When `constraints.maxWidth < 600`, it renders `_MobileWorkflowEditor`; otherwise it renders the existing `_DesktopCanvasEditor` (extracted inline widget). This is a single branch point, keeping both layouts in one file without additional routes.

_Alternative considered_: separate route or `AdaptiveLayout` from `flutter_adaptive_scaffold`. Rejected — adds a package dependency for a two-branch layout.

### D2 — Implicit linear edges; branching via inline child slots

On mobile, edges are not drawn by the user. Instead, `_buildGraph()` in mobile mode constructs edges by iterating the ordered node list: each node's `success`/`trigger` outcome points to the next node. Condition nodes expose two indented child slot columns (`true` / `false`) directly in the list card.

_Alternative considered_: keep explicit edge-drawing but use tap-to-select-source then tap-to-select-destination. Rejected — two-step interaction on a scrolling list is error-prone.

### D3 — `ReorderableListView` for node reordering

Mobile node cards sit inside a `ReorderableListView`. Long-press drag handle reorders; the list index becomes the edge order. This reuses a Flutter built-in without extra packages.

### D4 — Node configuration via existing bottom-sheet

`_NodePalette` and any future node-config sheets are surfaced via `showModalBottomSheet` — the same as the current add-node flow. No new UI surface needed for configuration.

### D5 — Shared state: `_WorkflowEditorScreenState` owns `_nodes` / `_edges`

Both desktop and mobile widgets are passed the same state callbacks (`onNodesChanged`, `onEdgesChanged`). The state owner (`_WorkflowEditorScreenState`) builds edges from the mobile card order before saving. This avoids duplicating save/validation logic.

## Risks / Trade-offs

- **Complex DAGs degrade gracefully** — If a workflow loaded from the server has edges that don't form a simple linear/branching chain, the mobile view falls back to rendering all nodes as a flat list (ignoring stored edge positions) and shows an informational banner. The graph is still editable and saveable. → Risk of silent edge loss is mitigated by showing the banner.
- **Breakpoint is physical-width only** — A tablet in landscape at 600 px triggers desktop mode, which is fine. A tablet in portrait at 599 px gets mobile view, which is also acceptable for touch. → No known mitigation needed; observe in practice.
- **`ReorderableListView` scroll conflicts** — Long-press drag can conflict with scroll momentum on some devices. → Use `ReorderableListView.builder` with explicit `proxyDecorator` and ensure drag handles are visually prominent.

## Implementation Notes

No migration required — this is a pure Flutter UI change with no data model or API impact. Ship in a single PR:

1. Extract canvas body into `_DesktopCanvasEditor` (rename, no logic change).
2. Implement `_MobileWorkflowEditor` with card list.
3. Add `LayoutBuilder` branch in `WorkflowEditorScreen.build()`.
4. Delete any code that no longer has a call site — do not leave dead code or compat wrappers.
5. Run `flutter analyze` + tests; add new widget tests for mobile layout.

## Interaction Flows

### 1. Tapping "Add step" → Node Palette Bottom Sheet

The FAB / "Add step" button opens the existing `_NodePalette` as a draggable bottom sheet (60 % height, scrollable).

```
┌────────────────────────────┐
│  ⚡ Trigger   Manual       │  ← current card list
│  ● HTTP Request            │
│  ✦ Key Check               │
│                            │
│    ┌─── [+ Add step] ──┐   │
│    └───────────────────┘   │
└────────────────────────────┘
          │ tap
          ▼
┌────────────────────────────┐  ╮
│          ━━━               │  │ drag handle
│  Add Node              ✕  │  │
├────────────────────────────┤  │
│  ● TRIGGERS                │  │
│  ┌─────────────────────┐   │  │
│  │ 👆 Manual           │   │  │
│  │  Triggered manually │   │  │
│  └─────────────────────┘   │  │ DraggableScrollableSheet
│  ┌─────────────────────┐   │  │ (initial 60 %, max 90 %)
│  │ 🔗 Webhook          │   │  │
│  │  Inbound HTTP hook  │   │  │
│  └─────────────────────┘   │  │
│  ┌─────────────────────┐   │  │
│  │ 🕐 Schedule         │   │  │
│  │  Cron schedule      │   │  │
│  └─────────────────────┘   │  │
│  ┌─────────────────────┐   │  │
│  │ 🔔 Event            │   │  │
│  │  Internal bus event │   │  │
│  └─────────────────────┘   │  │
│  ● ACTIONS                 │  │
│  ┌─────────────────────┐   │  │
│  │ 🤖 Assistant Turn   │   │  │
│  │  Run a prompt       │   │  │
│  └─────────────────────┘   │  │
│  ┌─────────────────────┐   │  │
│  │ 🌐 HTTP Request     │   │  │
│  │  Outbound HTTP call │   │  │
│  └─────────────────────┘   │  │
│  ● CONDITIONS              │  │
│  ┌─────────────────────┐   │  │
│  │ ✅ Always True      │   │  │
│  └─────────────────────┘   │  │
│  ┌─────────────────────┐   │  │
│  │ ❌ Always False     │   │  │
│  └─────────────────────┘   │  │
│  ┌─────────────────────┐   │  │
│  │ 🔑 Payload Key Check│   │  │
│  └─────────────────────┘   │  │
└────────────────────────────┘  ╯
          │ tap item
          ▼
  New node card appended
  at bottom of list
```

---

### 2. Tapping a Node Card → Config Bottom Sheet

Each card has an edit button (pencil icon). Tapping it opens `_NodeConfigSheet` — a modal bottom sheet whose fields depend on the node type. The sheet rises above the keyboard automatically.

```
CARD (idle)                         CARD (tap ✎)
┌══════════════════════╗            ┌══════════════════════╗
║ 🌐 HTTP Request      ║   ──✎──▶  ║ 🌐 HTTP Request      ║
║  http_request        ║            ║  http_request      ✎ ║  ← tapped
║                   ⠿ ║            ╚══════════════════════╝
╚══════════════════════╝                      │
                                              ▼
                              ┌────────────────────────────┐  ╮
                              │           ━━━              │  │
                              │  🌐 Configure: HTTP Request│  │
                              ├────────────────────────────┤  │
                              │  ┌──────────────────────┐  │  │
                              │  │ URL                  │  │  │
                              │  │ https://             │  │  │
                              │  └──────────────────────┘  │  │
                              │                            │  │
                              │  Method                    │  │
                              │  ┌────────┐               │  │
                              │  │ GET  ▾ │               │  │
                              │  └────────┘               │  │
                              │                            │  │
                              │  ╔══════════════════════╗  │  │
                              │  ║      [ Apply ]       ║  │  │
                              │  ╚══════════════════════╝  │  │
                              └────────────────────────────┘  ╯


Node type        Fields shown
───────────────  ────────────────────────────────────────────
manual           (no config needed — info label)
webhook          (no config needed — info label)
always_true      (no config needed — info label)
always_false     (no config needed — info label)
schedule         Cron expression  e.g. 0 * * * *
event            Event topic      e.g. message.completed
assistant_turn   Prompt           multiline text area
http_request     URL + Method     text + GET/POST/… dropdown
has_payload_key  Key name         text field
```

---

### 3. Full Mobile Editing Flow (end-to-end)

```
  [New Workflow]
       │
       ▼
┌────────────────────────┐
│ < Workflows    [Save]  │  AppBar (shared desktop/mobile)
│ Name: ________________ │
│ Desc: ________________ │
├────────────────────────┤
│                        │
│ ╔══════════════════╗   │
│ ║ ⚡ Manual Trigger║   │  ← pre-populated on "New"
│ ║              ✎ 🗑⠿║   │     ✎ edit  🗑 delete  ⠿ drag
│ ╚══════════════════╝   │
│         │              │
│         ▼              │
│  [+ Add step]          │  ← tap → palette sheet
│                        │
└────────────────────────┘
       │ add HTTP Request
       ▼
┌────────────────────────┐
│ ╔══════════════════╗   │
│ ║ ⚡ Manual Trigger║   │
│ ║              ✎ 🗑⠿║   │
│ ╚══════════════════╝   │
│         │              │
│         ▼              │
│ ╔══════════════════╗   │
│ ║ 🌐 HTTP Request  ║   │
│ ║  https://…    ✎ 🗑⠿║  │  ← tap ✎ → config sheet
│ ╚══════════════════╝   │
│         │              │
│         ▼              │
│  [+ Add step]          │
└────────────────────────┘
       │ tap [Save]
       ▼
  Graph JSON built:
  nodes: [trigger, action]
  edges: [trigger→action]  ← inferred from card order
```

## Open Questions

- Should condition-node branching on mobile expose both true/false sub-lists, or defer complex DAG editing to desktop only? → Lean towards simple linear-only on mobile for v1 with a "edit on desktop for branching" note.
