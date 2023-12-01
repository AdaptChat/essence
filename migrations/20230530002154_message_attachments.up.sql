CREATE TABLE IF NOT EXISTS attachments (
    id UUID NOT NULL PRIMARY KEY,
    message_id BIGINT NOT NULL,
    filename TEXT NOT NULL,
    size BIGINT NOT NULL,
    alt TEXT,
    CONSTRAINT fk_attachments_messages
        FOREIGN KEY (message_id)
        REFERENCES messages(id)
        ON DELETE CASCADE
);
