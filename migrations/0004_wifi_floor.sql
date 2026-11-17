CREATE TABLE wifi_floors (
    token TEXT NOT NULL REFERENCES workspaces(token) ON DELETE CASCADE,
    project INTEGER NOT NULL CHECK (project BETWEEN 0 AND 9),
    floor TEXT NOT NULL CHECK (json_valid(floor) AND length(floor) <= 200000),
    PRIMARY KEY (token, project)
) STRICT;
