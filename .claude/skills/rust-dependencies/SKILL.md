---
name: rust-dependencies
description: >
  Rust dependency management principles for this workspace. Use whenever adding, updating, removing,
  or reviewing dependencies in any Cargo.toml — including deciding which version to use, where to
  declare a dep, or auditing for duplicates.
---

## Rules

1. **Workspace-level declarations only.** All dependencies must be declared in `[workspace.dependencies]` in the root `Cargo.toml`. Never add a versioned dependency directly to a crate's `[dependencies]`.

2. **Crate Cargo.toml references workspace.** In each crate, reference workspace deps with:

   ```toml
   some-crate = { workspace = true }
   # with features:
   some-crate = { workspace = true, features = ["extra"] }
   ```

3. **Always use the latest version.** When adding a new dependency, look up and use the latest stable version on crates.io. Do not copy an old version from existing entries.

4. **Check before adding.** Before declaring a new dep in `[workspace.dependencies]`, grep the root `Cargo.toml` to confirm it isn't already there (possibly under a different feature set).

5. **No duplicate transitive deps.** Avoid adding a crate that is already pulled in transitively. Run `cargo tree -d` to surface duplicates. Prefer `cargo machete --with-metadata` to detect unused deps.

## Workflow

When adding a dependency:

1. Check root `Cargo.toml` `[workspace.dependencies]` — is it already declared?
2. If not, add it there with the latest version.
3. In the target crate's `Cargo.toml`, add `{ workspace = true }`.
4. After changes, run `make lint` to catch any issues.

When removing a dependency:

1. Remove the `{ workspace = true }` reference from the crate.
2. If no other crate uses it, remove the entry from `[workspace.dependencies]` too.
3. Run `cargo machete --with-metadata` to confirm nothing is left unused.
