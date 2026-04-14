## 1. Refactor: Extract Desktop Canvas Widget

- [x] 1.1 Extract the existing canvas body of `WorkflowEditorScreen` into a private `_DesktopCanvasEditor` stateless/stateful widget, passing `_nodes`, `_edges`, and all callbacks as parameters
- [x] 1.2 Verify `flutter analyze` passes and existing behaviour is unchanged after the extraction

## 2. Mobile Editor Widget

- [x] 2.1 Create `_MobileWorkflowEditor` stateful widget accepting `nodes`, `edges`, `onNodesChanged`, `onEdgesChanged`, and `onAddNode` callbacks
- [x] 2.2 Render nodes as `ReorderableListView` with one `_MobileNodeCard` per `EditorNode`
- [x] 2.3 Implement `_MobileNodeCard`: full-width `Card` showing node icon, colour accent, label, drag handle, and delete button
- [x] 2.4 Wire drag-handle reorder to update the `nodes` list order via `onNodesChanged`
- [x] 2.5 Wire delete button to remove the node and any edges referencing it via `onNodesChanged` / `onEdgesChanged`
- [x] 2.6 Add "Add step" `FilledButton` below the list that calls `onAddNode` (opening the existing `_NodePalette` sheet)

## 3. Edge Inference

- [x] 3.1 Add `_buildMobileEdges(List<EditorNode> nodes)` helper that iterates nodes in order and creates linear `EditorEdge` entries (trigger→action via `trigger` outcome; action→action via `success` outcome)
- [x] 3.2 Override `_buildGraph()` to use inferred edges when building in mobile mode (detect via same `< 600 px` check or a `_isMobileLayout` flag on state)

## 4. Complex DAG Banner

- [x] 4.1 Add `_isComplexDag(List<EditorNode> nodes, List<EditorEdge> edges)` utility: returns `true` if any node has more than one incoming edge, or if there are disconnected nodes
- [x] 4.2 Display a `MaterialBanner` / `Card` warning at the top of the mobile card list when `_isComplexDag` is true, with text "Complex graph — edit branching on a wider screen"

## 5. Layout Branch

- [x] 5.1 Wrap the `WorkflowEditorScreen` body in `LayoutBuilder`
- [x] 5.2 Render `_MobileWorkflowEditor` when `constraints.maxWidth < 600`, else render `_DesktopCanvasEditor`
- [x] 5.3 Ensure the top `AppBar` with name/description fields and Save button is shared between both layouts

## 6. Tests

- [x] 6.1 Add widget test: narrow viewport (width = 400) opens editor → `_MobileWorkflowEditor` is present, canvas is absent
- [x] 6.2 Add widget test: wide viewport (width = 800) opens editor → canvas is present, mobile list is absent
- [x] 6.3 Add widget test: add node in mobile view → card appears in list
- [x] 6.4 Add widget test: delete node in mobile view → card removed
- [x] 6.5 Add unit test: `_buildMobileEdges` with [trigger, action1, action2] → produces two edges in correct order
- [x] 6.6 Add unit test: `_isComplexDag` returns true for fan-in graph, false for linear chain

## 7. Visual Regression Baselines

- [x] 7.1 Capture a new Playwright baseline screenshot for the mobile card list editor at 390 px × 844 px viewport (3-node workflow: trigger + 2 actions)
- [x] 7.2 Capture a new Playwright baseline screenshot for the complex-DAG banner state at 390 px × 844 px viewport
- [x] 7.3 Verify the existing desktop canvas baseline is unchanged at 1280 px × 800 px viewport; update it only if the extraction refactor altered pixel output
- [x] 7.4 Commit all new and updated `.png` baseline files alongside the implementation

## 8. QA & Dead Code Removal

- [x] 8.1 Delete any code paths that are no longer reachable after the refactor — no compat wrappers, no unused overloads
- [x] 8.2 Run `flutter analyze` — zero issues
- [x] 8.3 Run `flutter test` — all tests pass
- [ ] 8.4 Manual smoke test: create, edit, and save a 3-node workflow in mobile emulator (Chrome DevTools device mode, 390 px width)
- [ ] 8.5 Manual smoke test: verify the desktop canvas is unaffected at 1280 px width
