CREATE TABLE IF NOT EXISTS push_registration_keys (
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    registration_key TEXT NOT NULL PRIMARY KEY,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE OR REPLACE FUNCTION delete_stale_keys() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
DECLARE row_count INT;
BEGIN
    DELETE FROM push_registration_keys
    WHERE created_at < CURRENT_TIMESTAMP - INTERVAL '2 days';
    IF found THEN
        GET DIAGNOSTICS row_count = ROW_COUNT;
        RAISE NOTICE 'DELETED % row(s) FROM push_registration_keys', row_count;
    END IF;
    RETURN NULL;
END;
$$;
