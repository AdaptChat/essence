ALTER TABLE channel_overwrites
    DROP CONSTRAINT channel_overwrites_target_id_fkey,
    DROP CONSTRAINT channel_overwrites_target_id_fkey1;

CREATE OR REPLACE FUNCTION on_channel_overwrites_target_delete()
    RETURNS TRIGGER AS $$
BEGIN
    DELETE FROM channel_overwrites WHERE target_id = OLD.id;
    RETURN OLD;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER on_channel_overwrites_member_delete BEFORE DELETE ON members
    FOR EACH ROW
EXECUTE FUNCTION on_channel_overwrites_target_delete();

CREATE TRIGGER on_channel_overwrites_role_delete BEFORE DELETE ON roles
    FOR EACH ROW
EXECUTE FUNCTION on_channel_overwrites_target_delete();