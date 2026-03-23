---
name: stimulus-playbook
description: >
  Practical Stimulus JS patterns for the assistant web UI. Covers controller
  naming, actions/targets/values, lifecycle usage, event-based coordination,
  htmx interoperability, and progressive enhancement guardrails. Use when an
  interaction is too stateful for pure HTML + htmx but does not need a full SPA
  framework.
license: MIT
---

# Stimulus Playbook

Use Stimulus as the lightest client-side layer for stateful behavior while
keeping server-rendered HTML and htmx as the primary architecture.

## When To Use

- UI state that lives in the browser (toggles, keyboard shortcuts, local filtering)
- Reusable behavior attached to many elements (menus, expandable cards)
- Cross-element coordination that is awkward with one-off event listeners

## When Not To Use

- Simple requests/swaps already handled cleanly with htmx attributes
- Purely presentational concerns that belong in CSS
- Data ownership that should remain server-side

## Component Creation Criteria

Use this decision gate before creating a Stimulus controller:

1. Can semantic HTML/CSS (or native elements like `<details>`) solve it? If yes,
   do not create a controller.
2. Is the need primarily server interaction and HTML replacement? Use htmx
   attributes/endpoints first.
3. Is there meaningful client-side state or reusable interaction behavior? Create
   a Stimulus controller.
4. Does one controller start mixing unrelated concerns? Split it into
   feature-scoped controllers.

## Conventions

- One controller per file, feature-scoped, default export
- File names: `*_controller.js`; identifier maps to kebab-case
- Prefer `static targets`, `static values`, and `static classes`
- Keep `connect()`/`disconnect()` cheap and idempotent
- Never inline JavaScript in templates

## Recommended Pattern

1. Start with semantic HTML and native elements.
2. Add htmx attributes for server interactions.
3. Add Stimulus only for local state and orchestration.
4. Communicate between controllers with events (`dispatch`) first.

## htmx Interop

- Listen to htmx lifecycle events in controller code (not `hx-on:*` inline JS)
- Reinitialize controller-local derived state after htmx swaps as needed
- Treat htmx responses as source of truth; Stimulus should adapt, not override

## Example

```html
<div
  data-controller="trace-filters"
  data-trace-filters-target="root"
  data-action="input->trace-filters#apply"
>
  <input data-trace-filters-target="query" type="search" />
  <ul data-trace-filters-target="list"></ul>
</div>
```

```js
import { Controller } from "@hotwired/stimulus";

export default class extends Controller {
  static targets = ["query", "list"];

  apply() {
    const q = this.queryTarget.value.trim().toLowerCase();
    for (const item of this.listTarget.querySelectorAll("li")) {
      item.hidden = !item.textContent.toLowerCase().includes(q);
    }
  }
}
```

## Review Checklist

- Is Stimulus solving real client-side state, not server rendering?
- Are controllers small, testable, and free of business logic?
- Are accessibility semantics preserved after behavior is added?
- Could this be done with native HTML or htmx only?
