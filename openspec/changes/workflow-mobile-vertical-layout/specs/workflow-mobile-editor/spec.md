## ADDED Requirements

### Requirement: Mobile layout activates below 600 px width

The system SHALL render a vertically-stacked card list editor (instead of the canvas) when the available widget width reported by `LayoutBuilder` is less than 600 logical pixels.

#### Scenario: Narrow viewport shows card list

- **WHEN** the workflow editor is opened on a device whose width is less than 600 px
- **THEN** a scrollable vertical list of node cards is displayed with no canvas visible

#### Scenario: Wide viewport shows canvas

- **WHEN** the workflow editor is opened on a device whose width is 600 px or greater
- **THEN** the existing free-form canvas editor is displayed unchanged

---

### Requirement: Node cards are strictly vertically aligned

Each node (trigger, action, condition) SHALL be rendered as a full-width card stacked top-to-bottom in document order, with no horizontal offset or canvas-position influence.

#### Scenario: Trigger appears first

- **WHEN** the mobile editor is displayed with a workflow containing a trigger and one action
- **THEN** the trigger card appears above the action card in the scroll list

#### Scenario: Cards span full width

- **WHEN** any node card is rendered in mobile view
- **THEN** the card width equals the available screen width minus horizontal padding (≥ 90% of screen width)

---

### Requirement: Nodes can be added via bottom-sheet palette

The system SHALL allow the user to add a new node by tapping an "Add step" button, which opens the existing `_NodePalette` bottom sheet. The new node is appended below the last node.

#### Scenario: Add step button appends a node

- **WHEN** the user taps "Add step" and selects a node type from the palette
- **THEN** a new card for that node type appears at the bottom of the card list

---

### Requirement: Nodes can be deleted from the card list

The system SHALL provide a delete control on each node card. Deleting a node removes it and any edges referencing it.

#### Scenario: Delete removes the card

- **WHEN** the user taps the delete icon on a node card and confirms
- **THEN** the card is removed from the list and is no longer present in the saved graph

---

### Requirement: Nodes can be reordered via drag handle

The system SHALL allow the user to reorder nodes by long-pressing a drag handle on each card and dragging it to a new position in the list.

#### Scenario: Drag handle reorders nodes

- **WHEN** the user long-presses the drag handle on a node card and drops it above another card
- **THEN** the dragged node appears in the new position and the card order reflects the new sequence

---

### Requirement: Edges are inferred from card order

The system SHALL automatically infer linear edges between cards (top → bottom) when building the graph JSON, without requiring the user to draw edges manually in mobile view.

#### Scenario: Saved graph contains inferred edges

- **WHEN** the user saves a workflow composed in mobile view with three nodes in order [T, A1, A2]
- **THEN** the persisted graph JSON contains edges T→A1 and A1→A2

---

### Requirement: Complex DAG shows desktop-only banner

The system SHALL display an informational banner in mobile view when the loaded workflow contains edges that cannot be represented as a simple linear chain (e.g., multiple incoming edges to one node, or disconnected subgraphs). The workflow remains viewable and saveable as a flat list.

#### Scenario: Non-linear graph triggers banner

- **WHEN** a workflow with a fan-in edge pattern is opened in mobile view
- **THEN** a banner reading "Complex graph — edit branching on a wider screen" is shown above the card list

---

### Requirement: Save and validation behave identically to desktop

The system SHALL apply the same validation rules (one trigger required, at least one node, name not empty) and the same save flow in mobile view as in desktop view.

#### Scenario: Missing trigger blocks save

- **WHEN** the user attempts to save a workflow in mobile view that has no trigger node
- **THEN** an error message "At least one trigger node is required" is shown and the save is blocked
