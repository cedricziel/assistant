-- Starter prompts shown in the empty chat for a Persona: a JSON array of strings.
ALTER TABLE personas ADD COLUMN starter_prompts TEXT NOT NULL DEFAULT '[]';
