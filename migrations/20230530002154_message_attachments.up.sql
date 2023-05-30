CREATE TABLE IF NOT EXISTS attachments (
    id UUID NOT NULL PRIMARY KEY,
    message_id BIGINT NOT NULL,
    revision_id BIGINT NOT NULL,
    filename TEXT NOT NULL,
    size BIGINT NOT NULL,
    alt TEXT,
    CONSTRAINT fk_attachments_messages
        FOREIGN KEY (message_id, revision_id)
        REFERENCES messages(id, revision_id)
        ON DELETE CASCADE
);
