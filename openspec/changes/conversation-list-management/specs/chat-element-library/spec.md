## MODIFIED Requirements

### Requirement: Conversation list view is pure

`ConversationListView` SHALL render a list of package `ConversationListItem`s (id, title, updatedAt, pinnedAt, archivedAt), an optional `header`, an optional `archivedSection` slot below the rows, and an optional `trailingBuilder` per item. Relative dates SHALL be computed against an injected `now` (defaulting to `DateTime.now()`). It SHALL emit `onSelect`. It SHALL highlight the row whose id equals the optional `selectedId`. When any item has a `pinnedAt`, it SHALL render a "Pinned" group header above those items. Deletion SHALL go through a single `Future<bool> Function(String id) onDelete` that the caller implements as confirm-then-delete; the row SHALL be dismissed only when it resolves to `true`, so a declined confirmation or a failed delete leaves the row in place, as `conversation_list.dart:123-140` does today.

Row actions SHALL be requested through `onRequestActions(ConversationActionRequest)`, carrying the `item`, the `PointerDeviceKind` of the pointer that started the gesture (`null` for keyboard activation), the global pointer `position` (`null` for the overflow button) and the overflow button's global `anchorRect` (`null` for row gestures). It SHALL fire on long-press, on secondary tap, and from a trailing overflow button that is shown while the row is hovered, focused or selected and carries the semantics label "Conversation actions". A start-to-end swipe SHALL emit `onTogglePin(item)` and keep the row in place; end-to-start swipe stays delete. The view SHALL NOT branch on platform and SHALL NOT perform I/O; choosing a menu or a sheet is the caller's job.

#### Scenario: Selection is delegated

- **WHEN** an item is tapped
- **THEN** `onSelect` SHALL fire with its id AND the view SHALL NOT navigate

#### Scenario: Declined delete keeps the item

- **GIVEN** `onDelete` resolves to `false` because the user declined
- **WHEN** the user swipes an item to delete
- **THEN** the item SHALL remain

#### Scenario: Failed delete keeps the item

- **GIVEN** `onDelete` resolves to `false` because the delete call failed
- **WHEN** the user swipes an item to delete
- **THEN** the item SHALL remain AND no dismissed `Dismissible` SHALL stay in the tree

#### Scenario: Successful delete dismisses the item

- **GIVEN** `onDelete` resolves to `true`
- **WHEN** the user swipes an item to delete
- **THEN** the row SHALL be dismissed

#### Scenario: Empty list

- **GIVEN** no items
- **WHEN** the view renders
- **THEN** the `header` SHALL render AND no item rows SHALL render

#### Scenario: Pinned group header

- **GIVEN** one item with `pinnedAt` and two without
- **WHEN** the view renders
- **THEN** a "Pinned" header SHALL precede the pinned row AND no header SHALL render when no item is pinned

#### Scenario: Selected row is highlighted and shows its overflow button

- **GIVEN** `selectedId` equals an item's id
- **WHEN** the view renders
- **THEN** that row SHALL be highlighted AND its overflow button SHALL be visible

#### Scenario: Gesture kind is reported

- **WHEN** the user long-presses a row with a touch pointer
- **THEN** `onRequestActions` SHALL fire once with `kind == PointerDeviceKind.touch` and the press position
- **AND** a secondary tap with a mouse SHALL report `kind == PointerDeviceKind.mouse`

#### Scenario: Swipe to pin keeps the row

- **WHEN** the user swipes a row start-to-end
- **THEN** `onTogglePin` SHALL fire once AND the row SHALL remain
