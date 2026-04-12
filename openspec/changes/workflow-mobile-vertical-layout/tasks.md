## 1. Refactor: Extract Desktop Canvas Widget

- [ ] 1.1 Extract the existing canvas body of `WorkflowEditorScreen` into a private `_DesktopCanvasEditor` stateless/stateful widget, passing `_nodes`, `_edges`, and all callbacks as parameters
- [ ] 1.2 Verify `flutter analyze` passes and existing behaviour is unchanged after the extraction

## 2. Mobile Editor Widget

- [ ] 2.1 Create `_MobileWorkflowEditor` stateful widget accepting `nodes`, `edges`, `onNodesChanged`, `onEdgesChanged`, and `onAddNode` callbacks
- [ ] 2.2 Render nodes as `ReorderableListView` with one `_MobileNodeCard` per `EditorNode`
- [ ] 2.3 Implement `_MobileNodeCard`: full-width `Card` showing node icon, colour accent, label, drag handle, and delete button
- [ ] 2.4 Wire drag-handle reorder to update the `nodes` list order via `onNodesChanged`
- [ ] 2.5 Wire delete button to remove the node and any edges referencing it via `onNodesChanged` / `onEdgesChanged`
- [ ] 2.6 Add "Add step" `FilledButton` below the list that calls `onAddNode` (opening the existing `_NodePalette` sheet)

## 3. Edge Inference

- [ ] 3.1 Add `_buildMobileEdges(List<EditorNode> nodes)` helper that iterates nodes in order and creates linear `EditorEdge` entries (trigger→action via `trigger` outcome; action→action via `success` outcome)
- [ ] 3.2 Override `_buildGraph()` to use inferred edges when building in mobile mode (detect via same `< 600 px` check or a `_isMobileLayout` flag on state)

## 4. Complex DAG Banner

- [ ] 4.1 Add `_isComplexDag(List<EditorNode> nodes, List<EditorEdge> edges)` utility: returns `true` if any node has more than one incoming edge, or if there are disconnected nodes
- [ ] 4.2 Display a `MaterialBanner` / `Card` warning at the top of the mobile card list when `_isComplexDag` is true, with text "Complex graph — edit branching on a wider screen"

## 5. Layout Branch

- [ ] 5.1 Wrap the `WorkflowEditorScreen` body in `LayoutBuilder`
- [ ] 5.2 Render `_MobileWorkflowEditor` when `constraints.maxWidth < 600`, else render `_DesktopCanvasEditor`
- [ ] 5.3 Ensure the top `AppBar` with name/description fields and Save button is shared between both layouts

## 6. Tests

- [ ] 6.1 Add widget test: narrow viewport (width = 400) opens editor → `_MobileWorkflowEditor` is present, canvas is absent
- [ ] 6.2 Add widget test: wide viewport (width = 800) opens editor → canvas is present, mobile list is absent
- [ ] 6.3 Add widget test: add node in mobile view → card appears in list
- [ ] 6.4 Add widget test: delete node in mobile view → card removed
- [ ] 6.5 Add unit test: `_buildMobileEdges` with [trigger, action1, action2] → produces two edges in correct order
- [ ] 6.6 Add unit test: `_isComplexDag` returns true for fan-in graph, false for linear chain

## 7. QA & Dead Code Removal

- [ ] 7.1 Delete any code paths that are no longer reachable after the refactor — no compat wrappers, no unused overloads
- [ ] 7.2 Run `flutter analyze` — zero issues
- [ ] 7.3 Run `flutter test` — all tests pass
- [ ] 7.4 Manual smoke test: create, edit, and save a 3-node workflow in mobile emulator (Chrome DevTools device mode, 390 px width)
- [ ] 7.5 Manual smoke test: verify the desktop canvas is unaffected at 1280 px width
