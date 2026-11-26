# Tool result render contracts

A tool can attach structured **render data** to its result so a client draws a
dedicated view (a terminal, a list of sources, ...) instead of a text blob. This
directory is the source of truth for those shapes. Rust (`assistant-core`,
`ToolRenderData`) and Dart (`assistant_ui`) both check their parsers against the
same fixtures.

```
schemas/tool-results/
  {kind}.schema.json            JSON Schema 2020-12
  fixtures/{kind}/valid-*.json     must pass the schema and parse into the model
  fixtures/{kind}/invalid-*.json   must fail the schema and parse to null
```

Contracts today: `terminal`, `sources`. More kinds follow in later changes.

## Rules

- **`kind` tags the payload.** `data` is a JSON object with a string `kind`
  naming the contract. Clients pick the renderer from `kind`, not from the tool
  name, because MCP tool names (`mcp__{server}__{tool}`) are not known ahead of
  time. Data without a `kind` is still carried and shown in a generic JSON view.
- **Evolve in place, no version field.** Top-level `additionalProperties` is
  `true`, so adding optional fields is safe. A client that meets a shape it
  cannot parse falls back to its generic view and never fails the event.
- **Drop, do not truncate.** The server caps serialized `data` at 32 KiB
  (`TOOL_RESULT_DATA_LIMIT_BYTES`). Over the cap the whole value is dropped and
  the text result stays. A JSON value is never cut in the middle.
- **Producers keep their own budgets.** Built-in tools stay well under the cap
  with per-contract limits that do not reuse the LLM-facing output limits, cut
  on a character boundary and set the contract's truncation flag:
  `terminal` tails at most 12 KiB per stream; `sources` at most 20 results with
  snippets of at most 300 characters.
- **Data is for clients only.** It never enters the LLM context; the model sees
  the tool's text content.
- **MCP servers can opt in.** An MCP tool that returns `structuredContent` with
  a `kind` from this directory gets the same rich view. The client's parse check
  is the safety boundary, so a payload that does not match simply falls back.

## Adding a contract

1. Add fixtures (valid and invalid) and watch
   `cargo test -p assistant-core --test tool_render_contracts` fail.
2. Add the `{kind}.schema.json`, the `ToolRenderData` variant and its `KINDS`
   and `schema()` entries.
3. Add the Dart renderer; its fixture test reads the same directory.
