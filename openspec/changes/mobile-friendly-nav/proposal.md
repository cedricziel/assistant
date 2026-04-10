## Why

The current navigation exposes all 9 destinations equally on mobile, creating a cramped bottom bar and burying the most-used screens (Chat, Personas, Skills) alongside rarely-used power-user items (Traces, Logs, Webhooks, Analytics). As the app matures into a daily-driver on phones and small tablets, this friction actively harms the primary workflows.

## What Changes

- Restructure bottom navigation to show only the 4–5 most frequently-used destinations on mobile
- Move developer/observability items (Traces, Logs, Webhooks, Analytics, Agents) into a secondary "More" overflow destination; Workflows stays primary
- Introduce a visual hierarchy difference between primary and secondary nav items on wide screens (nav rail stays full but groups items)
- Preserve full nav rail on desktop/tablet (≥768px) with a divider separating primary from developer tools

**Status Quo vs Proposed (ASCII art)**

```
STATUS QUO — Mobile (<768px)
┌─────────────────────────────────────┐
│              AppBar                 │
│  [≡ drawer]  Title          [...]  │
├─────────────────────────────────────┤
│                                     │
│           Screen Content            │
│                                     │
├─────────────────────────────────────┤
│ Chat│Trace│Logs│Person│Skills│...   │  ← 9 items crammed
│  🗨  │ ⏱  │ 📄 │  👤  │  🧩  │+4  │    (overflow hidden)
└─────────────────────────────────────┘

STATUS QUO — Desktop (≥768px)
┌──────┬──────────────────────────────┐
│  🗨  │         AppBar               │
│ Chat │                              │
│  ⏱  │                              │
│Trace │       Screen Content         │
│  📄  │                              │
│ Logs │                              │
│  👤  │                              │
│Perso │                              │
│  🧩  │                              │
│Skill │                              │
│  🔀  │                              │
│Workf │                              │
│  🔗  │                              │
│Webhk │                              │
│  🤖  │                              │
│Agent │                              │
│  📊  │                              │
│Analy │                              │
└──────┴──────────────────────────────┘


PROPOSED — Mobile (<768px)
┌─────────────────────────────────────┐
│              AppBar                 │
│              Title          [...]   │
├─────────────────────────────────────┤
│                                     │
│           Screen Content            │
│                                     │
├─────────────────────────────────────┤
│  Chat │ Contexts │ Skills │  More   │  ← 4 primary + overflow
│   🗨  │    👤    │   🧩   │  ···   │    "More" opens bottom sheet
└─────────────────────────────────────┘

"More" bottom sheet:
┌─────────────────────────────────────┐
│  Workflows     Webhooks             │
│  Agents        Analytics            │
│  Traces        Logs                 │
└─────────────────────────────────────┘


PROPOSED — Desktop (≥768px)
┌──────┬──────────────────────────────┐
│  🗨  │         AppBar               │
│ Chat │                              │
│  👤  │                              │
│ Ctxt │       Screen Content         │
│  🧩  │                              │
│Skill │                              │
│  🔀  │                              │
│Workfl│                              │
│──────│  ← divider                   │
│  ⏱  │                              │
│Trace │                              │
│  📄  │                              │
│ Logs │                              │
│  🔗  │                              │
│Webhk │                              │
│  🤖  │                              │
│Agent │                              │
│  📊  │                              │
│Analy │                              │
└──────┴──────────────────────────────┘
```

## Capabilities

### New Capabilities

- `mobile-nav-overflow`: Secondary "More" destination on mobile bottom bar that opens a modal bottom sheet listing all overflow destinations (Traces, Logs, Webhooks, Agents, Analytics)
- `nav-grouping`: Visual grouping/divider in the desktop navigation rail separating primary user-facing destinations from developer/power-user destinations

### Modified Capabilities

- (none — no existing specs are changing requirements)

## Impact

- `app/lib/shared/nav_shell.dart` — primary change surface; restructure destination list and add overflow sheet
- No backend, API, Rust, or router changes required
- No breaking changes to routes or deep links
