ALTER TABLE messages
    DROP COLUMN IF EXISTS revision_id;
ALTER TABLE attachments
    DROP COLUMN IF EXISTS revision_id;