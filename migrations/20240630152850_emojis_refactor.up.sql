ALTER TABLE emojis
    ALTER COLUMN created_by DROP NOT NULL,
    DROP CONSTRAINT emojis_created_by_fkey,
    ADD CONSTRAINT emojis_created_by_fkey FOREIGN KEY (created_by) REFERENCES users(id) ON DELETE SET NULL;