# Chat UI parity: implementation agent brief

Every agent that implements a chat-UI-parity PR follows this brief. Your task
prompt names one change and one PR id.

## Find your spec

The specs live on branch `t3/compare-client-ui` (spec PR #1207, not merged).
Read them without checking that branch out:

```sh
git fetch origin t3/compare-client-ui
git show origin/t3/compare-client-ui:openspec/changes/<change>/tasks.md
```

In `tasks.md`, find the section `## PR <id> —` and implement only its tasks.
Read the change's `proposal.md`, `design.md` and `specs/**` too. Also read
`openspec/chat-ui-parity-stack.md` (wave plan, migration numbers, hotspot
rules) and `openspec/chat-ui-parity-decisions.md` (final product decisions).

## Branch and scope

- Branch `chat-ui/<change-abbrev>-<id>` (lowercase, for example
  `chat-ui/mb-1`) off `origin/main`. Work only on that branch.
- Many sibling PRs are implemented at the same time by other agents. Do not
  implement work that belongs to another PR, even when it looks small.
- Do not edit the specs. If the spec is wrong about the code, adapt minimally
  and explain the deviation in the PR body.
- If a `Depends on:` PR has not merged yet, check with `gh pr list --search`
  whether it has an open PR. Stack on its branch only when your work cannot
  compile without it; otherwise branch from main.

## Space.db migrations

Use the number pre-allocated in `chat-ui-parity-stack.md` (for example
`046_message_tree`). Register it in both migration lists in
`crates/storage/src/lib.rs`. If main already uses that number, take the next
free one and say so in the PR body.

## Engineering rules

- TDD: write the failing test first, run it, and confirm it fails for the
  stated reason. Then write the minimum implementation, then refactor while
  the tests stay green.
- Follow `CLAUDE.md` and `AGENTS.md`: panic-free production Rust (no `unwrap`,
  `expect` or `panic`), `anyhow`, `tracing`, the import grouping rules, and no
  backward-compatibility shims.
- Use the toolchain pinned in `rust-toolchain.toml`. Build and test only the
  crates you touch (`-p <crate>`), never the whole workspace.
- Before each commit, run `cargo fmt --all` again after your last edit, then
  `cargo clippy -p <crates> -- -D warnings`, then the targeted
  `cargo test -p <crates>`. For Flutter, run `dart format`,
  `flutter analyze --fatal-infos` and the targeted `flutter test`.
- If you change HTTP routes or types, update `openapi.json`
  (`make dump-openapi`) and regenerate the Dart client
  (`make generate-flutter-client`, which needs Docker). Never hand-edit
  `app/packages/assistant_api`. If Docker is not available, say so in the PR
  body and leave the client regeneration as a follow-up commit.
- ADR PRs: follow the format of the existing files in `docs/adr/`.
- Review your own diff for reuse and simplification before committing.
- Keep the PR under about 500 changed lines. Use semantic commits with crate
  scope and proper grammar. End each commit message with
  `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`.

## Pull request

- Open a draft PR against `main` with `gh pr create --draft`. Use the PR's
  semantic title from `tasks.md`.
- In the body, write the sections Problem, Approach and Tests, then the line
  `Spec: openspec/changes/<change> PR <id> (#1207)`. Add a Deviations section
  if any. End the body with
  `🤖 Generated with [Claude Code](https://claude.com/claude-code)`.
- Wait for CI and fix failures. Judge only the newest run. Do not merge and do
  not mark the PR ready.

## Report

Give the PR URL, the head SHA and the CI state. Separate tests you ran and
saw pass from things you believe work. List spec deviations and any trades
you made.
