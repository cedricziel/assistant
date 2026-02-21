---
name: hello-wasm
description: >
  A minimal example WASM skill that returns a greeting. Use to verify that the
  WASM execution tier is working correctly.
license: MIT
metadata:
  tier: wasm
  mutating: "false"
  confirmation-required: "false"
  params: '{"name": {"type": "string", "description": "Name to greet (default: World)", "default": "World"}}'
---

## Instructions

Return a simple greeting from inside a WebAssembly plugin.

### Parameters

- `name` (string, optional, default `World`): The name to greet.

### Output format

Returns a single greeting line, e.g. `Hello, World! (from WASM)`.
