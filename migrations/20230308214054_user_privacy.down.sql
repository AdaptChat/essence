ALTER TABLE IF EXISTS users
    DROP COLUMN IF EXISTS dm_privacy,
    DROP COLUMN IF EXISTS group_dm_privacy,
    DROP COLUMN IF EXISTS friend_request_privacy;
