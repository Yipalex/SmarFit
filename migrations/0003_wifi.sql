ALTER TABLE workspaces ADD COLUMN wifi TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(wifi));
