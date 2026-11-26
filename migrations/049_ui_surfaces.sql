-- Migration 049: generative UI surfaces.
--
-- One row per A2UI surface the model rendered with `render-surface`
-- (openspec change generative-ui-a2ui, design D6). The row anchors to the
-- tool-call message that created it and holds the current components and
-- data model, so history reloads never replay tool arguments.
--
-- `status` is `active` until the user answers the surface, then `submitted`.
-- Closure by a later user turn is derived on read, so it has no status.
CREATE TABLE IF NOT EXISTS ui_surfaces (
    id                 TEXT    PRIMARY KEY,
    conversation_id    TEXT    NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    turn               INTEGER NOT NULL,
    anchor_message_id  TEXT    NOT NULL,
    anchor_call_index  INTEGER NOT NULL,
    tool_call_id       TEXT,
    title              TEXT    NOT NULL,
    fallback_text      TEXT    NOT NULL,
    catalog_id         TEXT    NOT NULL,
    protocol_version   TEXT    NOT NULL,
    components_json    TEXT    NOT NULL,
    data_model_json    TEXT    NOT NULL,
    send_data_model    INTEGER NOT NULL DEFAULT 1,
    status             TEXT    NOT NULL DEFAULT 'active'
                               CHECK (status IN ('active', 'submitted')),
    action_json        TEXT,
    action_message_id  TEXT,
    created_at         TEXT    NOT NULL,
    updated_at         TEXT    NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_ui_surfaces_conversation
    ON ui_surfaces (conversation_id);
