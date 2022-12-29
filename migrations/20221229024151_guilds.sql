CREATE TABLE IF NOT EXISTS guilds (
    id BIGINT NOT NULL PRIMARY KEY,
    owner_id BIGINT NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    icon TEXT,
    banner TEXT,
    vanity_url TEXT,
    flags INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (owner_id)
        REFERENCES users(id)
        ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS members (
    id BIGINT NOT NULL,
    guild_id BIGINT NOT NULL,
    nick TEXT,
    joined_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id, guild_id),
    FOREIGN KEY (id)
        REFERENCES users(id)
        ON DELETE CASCADE,
    FOREIGN KEY (guild_id)
        REFERENCES guilds(id)
        ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS roles (
    id BIGINT NOT NULL PRIMARY KEY,
    guild_id BIGINT NOT NULL,
    name TEXT NOT NULL,
    color INTEGER,
    position SMALLINT NOT NULL DEFAULT 1,
    gradient BOOLEAN NOT NULL DEFAULT FALSE,
    allowed_permissions BIGINT NOT NULL DEFAULT 0,
    denied_permissions BIGINT NOT NULL DEFAULT 0,
    flags INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (guild_id)
        REFERENCES guilds(id)
        ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS role_data (
    role_id BIGINT NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    guild_id BIGINT NOT NULL REFERENCES guilds(id) ON DELETE CASCADE,
    PRIMARY KEY (role_id, user_id, guild_id)
);

CREATE TABLE IF NOT EXISTS channels (
    id BIGINT NOT NULL PRIMARY KEY,
    guild_id BIGINT,
    type TEXT NOT NULL,
    name TEXT,
    position SMALLINT,
    parent_id BIGINT,
    topic TEXT,
    icon TEXT,
    slowmode INTEGER,
    nsfw BOOLEAN,
    locked BOOLEAN,
    user_limit SMALLINT,
    owner_id BIGINT,
    FOREIGN KEY (guild_id)
        REFERENCES guilds(id)
        ON DELETE CASCADE,
    FOREIGN KEY (parent_id)
        REFERENCES channels(id)
        ON DELETE SET NULL,
    FOREIGN KEY (owner_id)
        REFERENCES users(id)
        ON DELETE SET NULL
);

CREATE TABLE IF NOT EXISTS channel_overwrites (
    guild_id BIGINT NOT NULL,
    channel_id BIGINT NOT NULL,
    target_id BIGINT NOT NULL,
    allow BIGINT NOT NULL DEFAULT 0,
    deny BIGINT NOT NULL DEFAULT 0,
    PRIMARY KEY (guild_id, channel_id, target_id),
    FOREIGN KEY (guild_id)
        REFERENCES guilds (id)
        ON DELETE CASCADE,
    FOREIGN KEY (channel_id)
        REFERENCES channels (id)
        ON DELETE CASCADE,
    FOREIGN KEY (target_id)
        REFERENCES users (id)
        ON DELETE CASCADE,
    FOREIGN KEY (target_id)
        REFERENCES roles (id)
        ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS channel_recipients (
    channel_id BIGINT NOT NULL,
    user_id BIGINT NOT NULL,
    PRIMARY KEY (channel_id, user_id),
    FOREIGN KEY (channel_id)
        REFERENCES channels(id)
        ON DELETE CASCADE,
    FOREIGN KEY (user_id)
        REFERENCES users(id)
        ON DELETE CASCADE
);
