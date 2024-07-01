ALTER TABLE emojis
    DROP CONSTRAINT emojis_created_by_fkey,
    ADD CONSTRAINT emojis_created_by_fkey FOREIGN KEY (created_by) REFERENCES users(id) ON DELETE CASCADE,
    ALTER COLUMN created_by SET NOT NULL;
