CREATE TABLE IF NOT EXISTS emojis (
    id BIGINT NOT NULL PRIMARY KEY,
    guild_id BIGINT NOT NULL REFERENCES guilds(id) ON DELETE CASCADE,
    name VARCHAR(256) NOT NULL,
    created_by BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE -- user who created the emoji
);
