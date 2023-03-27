CREATE TABLE IF NOT EXISTS messages (
    id BIGINT NOT NULL,
    -- 0 is the equivalent of a NULL value,
    -- we use 0 for the fk constraint though
    revision_id BIGINT NOT NULL DEFAULT 0,
    channel_id BIGINT NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    author_id BIGINT REFERENCES users(id) ON DELETE SET NULL,
    type TEXT NOT NULL DEFAULT 'default',
    content TEXT,
    embeds JSONB NOT NULL DEFAULT '[]'::JSONB,
    flags INTEGER NOT NULL DEFAULT 0,
    stars INTEGER NOT NULL DEFAULT 0,
    metadata_user_id BIGINT REFERENCES users(id) ON DELETE SET NULL,
    metadata_pinned_message_id BIGINT,
    metadata_pinned_by BIGINT REFERENCES users(id) ON DELETE SET NULL,
    PRIMARY KEY (id, revision_id),
    FOREIGN KEY (metadata_pinned_message_id, revision_id)
        REFERENCES messages(id, revision_id)
        ON DELETE SET NULL
);