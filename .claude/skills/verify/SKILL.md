---
name: verify
description: >
  Run a throwaway assistant server with the freshly built web app and drive it
  in a browser to confirm a change works end to end. Use before opening a PR
  that changes user-visible behavior (web UI, sign-in, OAuth, routing), or when
  asked to verify, smoke-test or "see it working".
---

# Verify

Starts an isolated server (temporary `HOME`, free loopback port, no user
config) and checks behavior in the preview browser. Never point this at
`~/.assistant` or a live server.

## 1. Build

```sh
make build-web                 # app/build/web, embedded by the server
cargo build -p assistant-cli   # target/debug/assistant
```

Rebuild the web app after every Flutter change; `cargo build` alone embeds the
previous web build.

## 2. Start an instance

```sh
.claude/skills/verify/scripts/start-instance.sh
```

It prints `URL`, `HOME`, `PID`, `LOG` and `ADMIN_CREDENTIALS`. On first start
the server seeds org `default` with user `admin@localhost` and writes the
generated password to `ADMIN_CREDENTIALS` (`password: …` line).

## 3. Drive it

Use the preview tools in an **incognito** tab (`profileId: "incognito"`,
`reuseExistingTab: false`) so no session leaks between runs.

- Load the app from `URL/`, not `URL/login`: the server itself serves a legacy
  token form at exactly `/login`. In-app navigation to `/login` is the Flutter
  screen.
- Flutter renders to a canvas: the accessibility tree is mostly empty, so
  click by coordinates from a screenshot. Server-rendered pages
  (`/oauth/authorize`) have normal refs.
- Wait on URLs with `preview_wait_for` (`urlIncludes`) rather than sleeping.

## 4. Clean up, even on failure

```sh
.claude/skills/verify/scripts/stop-instance.sh <HOME>
```

Close the preview tab with `t3_preview_close`.

## Covered flows

| Flow                       | Steps                                                                          | Expect                                                                                                                  |
| -------------------------- | ------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------- |
| Web sign-in (password org) | `URL/` → **Sign in** → email `admin@localhost` → Continue → password → Sign In | `/oauth/register` 201, server email page, 303 to `/login/callback?code=…&state=…`, `/oauth/token` 200, lands on `/chat` |

Not covered yet: OIDC orgs (needs a local identity provider), native apps,
`assistant login` device approval. Add a row when you verify a new flow.
