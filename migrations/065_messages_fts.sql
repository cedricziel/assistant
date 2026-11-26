-- Migration 065: full-text index over user and assistant messages.
--
-- `messages_fts` is a contentful FTS5 table, not an external-content table
-- keyed on `messages.rowid`: `messages.id` is TEXT, so its implicit rowid may
-- be renumbered by `VACUUM` (and `assistant backup` snapshots with
-- `VACUUM INTO`). The FTS table's own rowid lives in shadow tables with an
-- INTEGER PRIMARY KEY, which VACUUM keeps. Rows are tied to messages by
-- `message_id`.
--
-- Indexed rows: role user/assistant, non-blank content, and not a thinking
-- step (`<think>…</think>`). Tool results, system rows, tool-call stubs and
-- reasoning text are never searched. The same predicate guards the insert
-- trigger, the re-insert of the update trigger and the backfill.
--
-- U+E000/U+E001 are the sentinels `snippet()` uses to mark hits; they are
-- replaced by a space in the indexed copy so a snippet can only contain
-- sentinels it inserted itself. The stored message is unchanged.
--
-- `messages_fts_map` records each message's FTS rowid. `message_id` is an
-- UNINDEXED column, so `DELETE FROM messages_fts WHERE message_id = ?` scans
-- the whole index (measured: ~3.5 s to cascade-delete 200 messages in a
-- 10k-message space). Deleting by rowid through the map is a point lookup.
--
-- The update trigger fires on `content` only, so other column updates
-- (parent_id, run_id, tool columns) do not churn the index. The delete trigger
-- has no predicate: deleting a row that was never indexed is a no-op on a
-- contentful table. `conversations` cascades fire it for every message.
CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(
    content,
    message_id UNINDEXED,
    conversation_id UNINDEXED,
    tokenize = 'unicode61 remove_diacritics 2'
);

CREATE TABLE IF NOT EXISTS messages_fts_map (
    message_id TEXT    PRIMARY KEY,
    fts_rowid  INTEGER NOT NULL
) WITHOUT ROWID;

CREATE TRIGGER IF NOT EXISTS messages_fts_after_insert
AFTER INSERT ON messages
WHEN new.role IN ('user', 'assistant')
 AND trim(new.content) <> ''
 AND new.content NOT LIKE '<think>%'
BEGIN
    INSERT INTO messages_fts (content, message_id, conversation_id)
    VALUES (
        replace(replace(new.content, char(57344), ' '), char(57345), ' '),
        new.id,
        new.conversation_id
    );
    INSERT INTO messages_fts_map (message_id, fts_rowid)
    VALUES (new.id, last_insert_rowid());
END;

CREATE TRIGGER IF NOT EXISTS messages_fts_after_update_content
AFTER UPDATE OF content ON messages
BEGIN
    DELETE FROM messages_fts
    WHERE rowid IN (SELECT fts_rowid FROM messages_fts_map WHERE message_id = old.id);
    DELETE FROM messages_fts_map WHERE message_id = old.id;
    INSERT INTO messages_fts (content, message_id, conversation_id)
    SELECT replace(replace(new.content, char(57344), ' '), char(57345), ' '),
           new.id,
           new.conversation_id
    WHERE new.role IN ('user', 'assistant')
      AND trim(new.content) <> ''
      AND new.content NOT LIKE '<think>%';
    INSERT INTO messages_fts_map (message_id, fts_rowid)
    SELECT new.id, last_insert_rowid()
    WHERE new.role IN ('user', 'assistant')
      AND trim(new.content) <> ''
      AND new.content NOT LIKE '<think>%';
END;

CREATE TRIGGER IF NOT EXISTS messages_fts_after_delete
AFTER DELETE ON messages
BEGIN
    DELETE FROM messages_fts
    WHERE rowid IN (SELECT fts_rowid FROM messages_fts_map WHERE message_id = old.id);
    DELETE FROM messages_fts_map WHERE message_id = old.id;
END;

-- Backfill. The FTS rowid is the message's current rowid: unique within the
-- new table, and later inserts continue from the highest one. The map, not the
-- rowid's origin, is the link from then on.
INSERT INTO messages_fts (rowid, content, message_id, conversation_id)
SELECT rowid,
       replace(replace(content, char(57344), ' '), char(57345), ' '),
       id,
       conversation_id
FROM messages
WHERE role IN ('user', 'assistant')
  AND trim(content) <> ''
  AND content NOT LIKE '<think>%';

INSERT INTO messages_fts_map (message_id, fts_rowid)
SELECT id, rowid
FROM messages
WHERE role IN ('user', 'assistant')
  AND trim(content) <> ''
  AND content NOT LIKE '<think>%';
