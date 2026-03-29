-- Migration 028: add body_text column to skills table
--
-- Stores the full SKILL.md body (markdown content after frontmatter) in the
-- database so the web UI can pre-populate edit forms without reading from disk.

ALTER TABLE skills ADD COLUMN body_text TEXT NOT NULL DEFAULT '';
