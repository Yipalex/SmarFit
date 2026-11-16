ALTER TABLE workspaces ADD COLUMN questionnaire_key TEXT;
ALTER TABLE workspaces ADD COLUMN inbox TEXT NOT NULL DEFAULT '[]' CHECK (json_valid(inbox));
CREATE UNIQUE INDEX workspaces_questionnaire_key ON workspaces(questionnaire_key)
    WHERE questionnaire_key IS NOT NULL;
