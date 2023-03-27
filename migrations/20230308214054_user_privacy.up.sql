ALTER TABLE IF EXISTS users
    ADD COLUMN IF NOT EXISTS dm_privacy SMALLINT NOT NULL DEFAULT 7, -- friends | mutual_friends | guild_members
    ADD COLUMN IF NOT EXISTS group_dm_privacy SMALLINT NOT NULL DEFAULT 1, -- friends
    ADD COLUMN IF NOT EXISTS friend_request_privacy SMALLINT NOT NULL DEFAULT 8; -- everyone