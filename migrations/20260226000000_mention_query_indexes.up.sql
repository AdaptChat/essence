CREATE INDEX IF NOT EXISTS idx_messages_channel_message ON messages (channel_id, id);
CREATE INDEX IF NOT EXISTS idx_role_data_user_guild ON role_data (user_id, guild_id);
CREATE INDEX IF NOT EXISTS idx_messages_mentions ON messages USING GIN (mentions);
