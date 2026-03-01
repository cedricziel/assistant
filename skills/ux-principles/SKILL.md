---
name: ux-principles
description: >
  Web UI accessibility and UX patterns for the assistant web UI. Covers
  semantic HTML, ARIA attributes, keyboard navigation, focus management,
  responsive breakpoints, form validation, and dark-theme colour tokens.
  Use when building or reviewing any web-facing HTML/CSS in this project.
license: MIT
metadata:
  tier: info
  mutating: "false"
  confirmation-required: "false"
---

# UX Principles

Guidelines for building accessible, consistent, and responsive pages in the
assistant web UI. All pages are server-rendered Askama templates extending
`base.html`, styled with a shared fingerprinted CSS bundle, and enhanced
with htmx for partial updates.

## Semantic HTML

Use the correct element for the job. Screen readers and keyboard users
depend on it.

| Intent         | Element                                      | Notes                                     |
| -------------- | -------------------------------------------- | ----------------------------------------- |
| Page sections  | `<main>`, `<header>`, `<aside>`              | Already in `base.html`                    |
| Navigation     | `<nav aria-label="...">`                     | Icon rail, bottom tabs, drawer all use it |
| Lists of items | `<ul>`/`<ol>` + `<li>`                       | Trace list, log list, agent list          |
| Data tables    | `<table>` + `<thead>`/`<tbody>`              | Use `scope="col"` on `<th>`               |
| Actions        | `<button>` for actions, `<a>` for navigation | Never `<div onclick>`                     |
| Forms          | `<form>` + `<label for="id">`                | Every input needs a visible label         |
| Headings       | `<h1>`..`<h6>` in order                      | One `<h1>` per page, no skipped levels    |

## ARIA Attributes

Add ARIA only when native semantics are insufficient.

```html
<!-- Alert for form errors -->
<div class="login-error" role="alert">{{ error }}</div>

<!-- Link input to its error message -->
<input aria-describedby="login-error" ... />

<!-- Label for icon-only navigation -->
<nav aria-label="Main navigation">
  <button aria-label="Menu"></button>
</nav>
```

**Rules:**

- Every icon-only button or link needs `aria-label` or `title`
- Error messages use `role="alert"` so screen readers announce them
- `aria-describedby` links inputs to their error/help text
- `aria-current="page"` on the active nav item (the `.active` class is visual only)

## Keyboard Navigation

All interactive elements must be keyboard-accessible.

```html
<!-- Clickable card pattern -->
<div
  class="trace-card is-clickable"
  tabindex="0"
  onclick="window.location='/traces/{{ id }}'"
  onkeydown="if(event.key==='Enter')this.click()"
></div>
```

**Checklist:**

- `tabindex="0"` on any non-native interactive element (clickable cards, custom toggles)
- `onkeydown` handler for Enter/Space on clickable divs
- `.is-clickable` class for `cursor: pointer` + hover effect
- Tab order follows visual order (no `tabindex > 0`)
- Focus ring visible on all focusable elements (the default CSS includes `:focus-visible` styles)
- Modals/drawers trap focus and return it on close

## Responsive Breakpoints

The app uses three tiers, tested with Playwright at each viewport:

| Tier    | Width     | Layout                               |
| ------- | --------- | ------------------------------------ |
| Desktop | >= 900px  | Icon rail (left) + top bar + content |
| Tablet  | 640-899px | Hamburger menu + slide-over drawer   |
| Mobile  | < 640px   | Bottom tab bar + stacked content     |

**Rules:**

- Desktop icon rail: `display: flex` by default, hidden below 900px
- Hamburger + drawer: hidden on desktop, visible on tablet
- Bottom tabs: hidden above 640px, visible on mobile
- Content area: single column on mobile, can use grid on desktop
- Touch targets: minimum 44x44px on mobile (iOS HIG recommendation)
- Use `viewport-fit=cover` for safe-area-inset (notched devices)

## Form Patterns

### Server-side validation (standard)

```html
<form method="POST" action="/webhooks">
  <label for="url">URL</label>
  <input type="url" id="url" name="url" required placeholder="https://..." />
  {% if let Some(error) = url_error %}
  <div class="field-error" role="alert">{{ error }}</div>
  {% endif %}
  <button type="submit">Save</button>
</form>
```

### Client-side pre-validation (progressive enhancement)

```html
<textarea id="skills_json" oninput="validateJson(this)"></textarea>
<div id="skills-json-error" class="field-error" aria-live="polite"></div>

<script>
  function validateJson(el) {
    const errEl = document.getElementById("skills-json-error");
    try {
      JSON.parse(el.value || "[]");
      errEl.textContent = "";
    } catch (e) {
      errEl.textContent = "Invalid JSON: " + e.message;
    }
  }
</script>
```

**Rules:**

- Always validate server-side; client-side is progressive enhancement only
- Use `required`, `type="url"`, `type="email"`, `pattern` attributes
- Error text uses `role="alert"` or `aria-live="polite"`
- Submit button inside the `<form>` element (not detached)

## Colour Tokens (Dark Theme)

The app uses a consistent dark palette. Reference these values from `base.css`:

| Token                | Value                  | Usage                         |
| -------------------- | ---------------------- | ----------------------------- |
| Background (body)    | `#020611`              | Page background               |
| Surface              | `#0a1628`              | Cards, panels                 |
| Surface border       | `#0b1b32`              | Card borders                  |
| Text primary         | `#e5e9f0`              | Headings, body text           |
| Text secondary       | `#8aa5d8`              | Labels, metadata              |
| Accent blue          | `#6ec6ff`              | Links, focus rings            |
| Accent brand         | `#7aa2ff`              | Brand text, active indicators |
| Primary button       | `#2563eb`              | CTA buttons                   |
| Primary button hover | `#1d4ed8`              | Button hover state            |
| Error background     | `rgba(239,68,68,0.15)` | Error banners                 |
| Error border         | `rgba(239,68,68,0.3)`  | Error borders                 |
| Error text           | `#fca5a5`              | Error message text            |
| Success              | `#4ade80`              | Status badges                 |
| Warning              | `#fbbf24`              | Warning indicators            |

**Rules:**

- Never use pure white (`#fff`) for text — use `#e5e9f0`
- Never use pure black (`#000`) for backgrounds — use `#020611`
- Maintain minimum 4.5:1 contrast ratio for text (WCAG AA)
- Use `rgba()` with low alpha for translucent overlays (drawer backdrop, error backgrounds)

## Empty States

Every list page must handle the empty case gracefully:

```html
{% if items.is_empty() %}
<div class="empty-state">
  <p>No traces recorded yet.</p>
  <p class="empty-hint">
    Traces appear here when the agent processes requests.
  </p>
</div>
{% else %} ... {% endif %}
```

**Rules:**

- Explain _what_ would appear and _how_ to make it appear
- Use muted text colour (`#8aa5d8`)
- Centre vertically in the content area on desktop
- Don't show a sad face or error icon — this is a normal starting state

## Action Bars

Use semantic `<nav>` for groups of page-level actions:

```html
<nav class="action-bar" aria-label="Webhook actions">
  <a href="/webhooks/{{ id }}/edit" class="btn">Edit</a>
  <form method="POST" action="/webhooks/{{ id }}/toggle" style="display:inline">
    <button type="submit" class="btn">
      {% if webhook.is_active %}Deactivate{% else %}Activate{% endif %}
    </button>
  </form>
</nav>
```

**Rules:**

- Destructive actions (delete) use a distinct style (red text/border)
- Confirmation for destructive actions via `onclick="return confirm(...)"`
- Actions that change state use `<form method="POST">` (not GET links)
- Group related actions in a `<nav>` with `aria-label`
