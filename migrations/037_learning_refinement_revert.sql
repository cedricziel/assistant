-- Migration 037: add previous_skill_md column and expand status CHECK to include
-- 'confirmed' and 'reverted' for the revert-on-regression workflow.

-- SQLite does not support ALTER CHECK, so we recreate the table.

CREATE TABLE skill_refinements_new (
    id                  TEXT PRIMARY KEY,
    target_skill        TEXT NOT NULL,
    proposed_skill_md   TEXT NOT NULL,
    rationale           TEXT NOT NULL,
    status              TEXT NOT NULL DEFAULT 'pending'
                        CHECK(status IN ('pending', 'accepted', 'rejected', 'confirmed', 'reverted')),
    review_note         TEXT,
    previous_skill_md   TEXT,
    created_at          DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    reviewed_at         DATETIME
);

INSERT INTO skill_refinements_new (id, target_skill, proposed_skill_md, rationale, status, review_note, created_at, reviewed_at)
    SELECT id, target_skill, proposed_skill_md, rationale, status, review_note, created_at, reviewed_at
    FROM skill_refinements;

DROP TABLE skill_refinements;
ALTER TABLE skill_refinements_new RENAME TO skill_refinements;

CREATE INDEX IF NOT EXISTS idx_refinements_skill ON skill_refinements(target_skill, status);
