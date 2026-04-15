---
name: openapi-sync
description: Enforces OpenAPI spec discipline when working on REST API endpoints in this project. Triggers whenever adding, modifying, or removing HTTP routes, request/response types, or API handlers in the Rust web-ui crate (`crates/web-ui`). Reminds the agent to (1) update the committed `openapi.json` spec, (2) run `make dump-openapi` to re-export the spec from the running server, and (3) run `make generate-flutter-client` to regenerate the Dart/dio client in `app/packages/assistant_api/`. Also applies when changing route parameters, status codes, or authentication on existing endpoints.
---

# OpenAPI Sync Discipline

Any change to the REST API **must** keep `openapi.json` and the generated Flutter client in sync. The Flutter app's `app/packages/assistant_api/` package is **generated code — never edit it manually**.

## Mandatory Steps After Any API Change

1. **Make the server compile and run** so the spec can be exported.

2. **Re-export the OpenAPI spec:**

   ```sh
   make dump-openapi
   ```

   Calls the running server to produce a fresh `openapi.json` at the repo root.

3. **Regenerate the Flutter Dart/dio client:**

   ```sh
   make generate-flutter-client
   ```

   Requires `openapi-generator` (`brew install openapi-generator`).

4. **Commit both** `openapi.json` and the regenerated files in `app/packages/assistant_api/` together with the API change in the same commit (or a follow-up commit in the same PR).

## What Counts as an API Change

- Adding, removing, or renaming a route
- Changing request body or query parameter types
- Changing response body types or status codes
- Adding or removing authentication/authorization on an endpoint
- Changing streaming vs. non-streaming behavior

## Checklist Before Committing

- [ ] `openapi.json` at repo root reflects the new/changed endpoint
- [ ] `app/packages/assistant_api/` has been regenerated via `make generate-flutter-client`
- [ ] `flutter analyze` passes (`cd app && flutter analyze`)
- [ ] No manual edits inside `app/packages/assistant_api/`
