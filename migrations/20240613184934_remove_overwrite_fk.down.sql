ALTER TABLE channel_overwrites
    ADD CONSTRAINT channel_overwrites_target_id_fkey FOREIGN KEY (target_id) REFERENCES users (id) ON DELETE CASCADE,
    ADD CONSTRAINT channel_overwrites_target_id_fkey1 FOREIGN KEY (target_id) REFERENCES roles (id) ON DELETE CASCADE;

DROP FUNCTION IF EXISTS on_channel_overwrites_target_delete CASCADE;
DROP TRIGGER IF EXISTS on_channel_overwrites_member_delete ON members;
DROP TRIGGER IF EXISTS on_channel_overwrites_role_delete ON roles;