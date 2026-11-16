CREATE TABLE workspaces (
    token TEXT PRIMARY KEY,
    revision INTEGER NOT NULL CHECK (revision >= 1),
    document TEXT NOT NULL CHECK (json_valid(document))
) STRICT;
