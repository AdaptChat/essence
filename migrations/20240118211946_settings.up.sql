CREATE TABLE IF NOT EXISTS notification_settings (
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    target_id BIGINT NOT NULL, -- could be channel id (for any channel, INCLUDING dm channels), user id (to mute ALL messages for the user), or guild id
    notif_flags SMALLINT NOT NULL DEFAULT 0,
    PRIMARY KEY (user_id, target_id)
);

ALTER TABLE users ADD COLUMN IF NOT EXISTS settings INTEGER NOT NULL DEFAULT 0;
