CREATE TABLE IF NOT EXISTS attachments (
    id UUID NOT NULL PRIMARY KEY,
    message_id BIGINT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    filename TEXT NOT NULL,
    size BIGINT NOT NULL,
    alt TEXT
);
