use std::collections::HashMap;

use super::DbExt;
use crate::http::user::EditBotPayload;
use crate::{
    db::get_pool,
    error::UserInteractionType,
    http::user::EditUserPayload,
    models::{
        Bot, BotFlags, ClientUser, NotificationFlags, Permissions, PrivacyConfiguration,
        Relationship, RelationshipType, Settings, User, UserFlags, UserOnboardingFlags,
    },
    Error, NotFoundExt,
};

macro_rules! construct_user {
    ($data:ident) => {{
        User {
            id: $data.id as _,
            username: $data.username,
            display_name: $data.display_name as _,
            avatar: $data.avatar,
            banner: $data.banner,
            bio: $data.bio,
            flags: UserFlags::from_bits_truncate($data.flags as _),
        }
    }};
}

macro_rules! fetch_user {
    ($self:ident, $query:literal, $($arg:expr),* $(,)?) => {{
        let result = sqlx::query!($query, $($arg),*)
            .fetch_optional($self.executor())
            .await?
            .map(|r| construct_user!(r));

        Ok(result)
    }};
}

macro_rules! fetch_client_user {
    ($self:ident, $where:literal, $($arg:expr),* $(,)?) => {{
        let mut result = sqlx::query!("SELECT * FROM users WHERE " + $where, $($arg),*)
            .fetch_optional($self.executor())
            .await?
            .map(|r| ClientUser {
                user: construct_user!(r),
                email: r.email,
                password: r.password,
                dm_privacy: PrivacyConfiguration::from_bits_truncate(r.dm_privacy),
                group_dm_privacy: PrivacyConfiguration::from_bits_truncate(r.group_dm_privacy),
                friend_request_privacy: PrivacyConfiguration::from_bits_truncate(
                    r.friend_request_privacy,
                ),
                onboarding_flags: UserOnboardingFlags::from_bits_truncate(r.onboarding_flags),
                settings: Settings::from_bits_truncate(r.settings),
                notification_override: HashMap::new(),
            });

        if let Some(client) = result.as_mut() {
            client.notification_override =
                sqlx::query!(
                    "SELECT target_id, notif_flags FROM notification_settings WHERE user_id = $1",
                    client.id as i64,
                )
                .fetch_all($self.executor())
                .await?
                .into_iter()
                .map(|r| (r.target_id as u64, NotificationFlags::from_bits_truncate(r.notif_flags)))
                .collect();
        }

        Ok(result)
    }};
}

macro_rules! query_relationships {
    ($where:literal, $($arg:expr),* $(,)?) => {{
        sqlx::query_as!(
            DbRelationship,
            r#"
            SELECT
                r.target_id,
                u.username AS username,
                u.display_name AS display_name,
                u.avatar AS avatar,
                u.banner AS banner,
                u.bio AS bio,
                u.flags AS flags,
                r.type AS "kind: _"
            FROM
                relationships AS r
            INNER JOIN
                users AS u ON u.id = r.target_id
            WHERE "# + $where,
            $($arg),*
        )
    }};
}

macro_rules! privacy_configuration_method {
    ($(#[$doc:meta])* $meth_name:ident, $col:literal) => {
        $(#[$doc])*
        #[doc = ""]
        #[doc = "# Errors"]
        #[doc = "* If an error occurs with fetching the privacy configuration."]
        fn $meth_name<'slf, 'fut>(&'slf self, user_id: u64) ->
            ::core::pin::Pin<Box<dyn ::core::future::Future<Output = crate::Result<PrivacyConfiguration>> + Send + 'fut>>
        where
            'slf: 'fut,
            Self: Sync + 'fut,
        {
            Box::pin(async move {
                let privacy = sqlx::query!(
                    "SELECT " + $col + " AS col FROM users WHERE id = $1",
                    user_id as i64,
                )
                .fetch_optional(self.executor())
                .await?
                .ok_or_not_found("user", "user not found")?
                .col;

                Ok(PrivacyConfiguration::from_bits_truncate(privacy))
            })
        }
    };
}

macro_rules! query_bots {
    ($where:literal, $($arg:expr),* $(,)?) => {{
        sqlx::query!(
            r#"SELECT
                u.id AS id,
                u.username AS username,
                u.display_name AS display_name,
                u.avatar AS avatar,
                u.banner AS banner,
                u.bio AS bio,
                u.flags AS flags,
                b.owner_id AS owner_id,
                b.default_permissions AS default_permissions,
                b.flags AS bot_flags
            FROM
                users AS u
            INNER JOIN
                bots AS b ON u.id = b.user_id
            WHERE "# + $where,
            $($arg),*
        )
    }};
}

macro_rules! construct_bot {
    ($data:ident) => {{
        Bot {
            user: User {
                id: $data.id as _,
                username: $data.username,
                display_name: $data.display_name,
                avatar: $data.avatar,
                banner: $data.banner,
                bio: $data.bio,
                flags: UserFlags::from_bits_truncate($data.flags as _),
            },
            owner_id: $data.owner_id as _,
            default_permissions: Permissions::from_bits_truncate($data.default_permissions),
            flags: BotFlags::from_bits_truncate($data.bot_flags as _),
        }
    }};
}

#[derive(Copy, Clone, sqlx::Type)]
#[sqlx(type_name = "relationship_type")] // only for PostgreSQL to match a type definition
#[sqlx(rename_all = "snake_case")]
pub enum DbRelationshipType {
    Friend,
    Incoming,
    Outgoing,
    Blocked,
}

pub struct DbRelationship {
    pub target_id: i64,
    pub username: String,
    pub display_name: Option<String>,
    pub avatar: Option<String>,
    pub banner: Option<String>,
    pub bio: Option<String>,
    pub flags: i32,
    pub kind: DbRelationshipType,
}

#[async_trait::async_trait]
pub trait UserDbExt<'t>: DbExt<'t> {
    /// Fetches a user from the database with the given ID.
    ///
    /// # Errors
    /// * If an error occurs with fetching the user. If the user is not found, `Ok(None)` is
    /// returned.
    async fn fetch_user_by_id(&self, id: u64) -> sqlx::Result<Option<User>> {
        fetch_user!(self, "SELECT * FROM users WHERE id = $1", id as i64)
    }

    /// Fetches a user from the database with the given username this is case-insensitive.
    ///
    /// # Errors
    /// * If an error occurs with fetching the user. If the user is not found, `Ok(None)` is
    /// returned.
    async fn fetch_user_by_username(&self, username: &str) -> sqlx::Result<Option<User>> {
        fetch_user!(
            self,
            "SELECT * FROM users WHERE LOWER(username) = LOWER($1)",
            username
        )
    }

    /// Fetches the client user from the database.
    ///
    /// # Errors
    /// * If an error occurs with fetching the client user.
    async fn fetch_client_user_by_id(&self, id: u64) -> sqlx::Result<Option<ClientUser>> {
        fetch_client_user!(self, "id = $1", id as i64)
    }

    /// Fetches the flags of a user by their user ID.
    ///
    /// # Errors
    /// * If an error occurs with fetching the user.
    async fn fetch_user_flags_by_id(&self, id: u64) -> sqlx::Result<Option<UserFlags>> {
        Ok(
            sqlx::query!("SELECT flags FROM users WHERE id = $1", id as i64)
                .fetch_optional(self.executor())
                .await?
                .map(|r| UserFlags::from_bits_truncate(r.flags as _)),
        )
    }

    /// Sets the flags of a user by their user ID.
    ///
    /// # Note
    /// This method uses transactions, on the event of an ``Err`` the transaction must be properly
    /// rolled back, and the transaction must be committed to save the changes.
    ///
    /// # Errors
    /// * If an error occurs with the database.
    async fn set_user_flags_by_id(&mut self, id: u64, flags: UserFlags) -> sqlx::Result<()> {
        sqlx::query!(
            "UPDATE users SET flags = $1 WHERE id = $2",
            flags.bits() as i32,
            id as i64
        )
        .execute(self.transaction())
        .await?;

        Ok(())
    }

    /// Fetches the client user from the database by email.
    ///
    /// # Errors
    /// * If an error occurs with fetching the client user. If the user is not found, `Ok(None)` is
    /// returned.
    async fn fetch_client_user_by_email(
        &self,
        email: impl AsRef<str> + Send,
    ) -> sqlx::Result<Option<ClientUser>> {
        fetch_client_user!(self, "email = $1", email.as_ref())
    }

    /// Returns `true` if the given email is taken.
    ///
    /// # Errors
    /// * If an error occurs with the database.
    async fn is_email_taken(&self, email: impl AsRef<str> + Send) -> sqlx::Result<bool> {
        let result = sqlx::query!(
            "SELECT EXISTS(SELECT 1 FROM users WHERE email = $1)",
            email.as_ref()
        )
        .fetch_one(self.executor())
        .await?
        .exists;

        Ok(result.unwrap())
    }

    /// Returns `true` if the given username is taken, excluding the user with the given ID.
    ///
    /// # Errors
    /// * If an error occurs with the database.
    async fn is_username_taken_excluding(
        &self,
        username: impl AsRef<str> + Send,
        exclude: u64,
    ) -> sqlx::Result<bool> {
        let result = sqlx::query!(
            "SELECT EXISTS(SELECT 1 FROM users WHERE LOWER(username) = LOWER($1) AND id != $2)",
            username.as_ref(),
            exclude as i64,
        )
        .fetch_one(self.executor())
        .await?
        .exists;

        Ok(result.unwrap())
    }

    /// Returns `true` if the given username is taken.
    ///
    /// # Errors
    /// * If an error occurs with the database.
    async fn is_username_taken(&self, username: impl AsRef<str> + Send) -> sqlx::Result<bool> {
        let result = sqlx::query!(
            "SELECT EXISTS(SELECT 1 FROM users WHERE LOWER(username) = LOWER($1))",
            username.as_ref()
        )
        .fetch_one(self.executor())
        .await?
        .exists;

        Ok(result.unwrap())
    }

    /// Registers a user in the database with the given payload. No validation is done, they must
    /// be done before calling this method. Hashing of the password is done here.
    ///
    /// # Note
    /// This method uses transactions, on the event of an ``Err`` the transaction must be properly
    /// rolled back, and the transaction must be committed to save the changes.
    ///
    /// # Errors
    /// * If an error occurs with registering the user.
    #[cfg(feature = "auth")]
    async fn register_user(
        &mut self,
        id: u64,
        username: impl AsRef<str> + Send,
        display_name: Option<impl AsRef<str> + Send>,
        email: impl AsRef<str> + Send,
        password: impl AsRef<str> + Send,
    ) -> crate::Result<()> {
        let password = password.as_ref();
        let hashed = crate::auth::hash_password(password).await?;

        sqlx::query!(
            "INSERT INTO
                users (id, username, display_name, email, password)
            VALUES
                ($1, $2, $3, $4, $5)",
            id as i64,
            username.as_ref().trim(),
            display_name.as_ref().map(|s| s.as_ref().trim()),
            email.as_ref().trim(),
            hashed,
        )
        .execute(self.transaction())
        .await?;

        Ok(())
    }

    /// Edits a user in the database with the given payload. No validation is done, they must
    /// be done before calling this method. Returns `(old_user, new_user)`.
    ///
    /// # Note
    /// This method uses transactions, on the event of an ``Err`` the transaction must be properly
    /// rolled back, and the transaction must be committed to save the changes.
    ///
    /// # Errors
    /// * If an error occurs with editing the user.
    /// * If the user is not found.
    /// * If the user is trying to change their username to one that is already taken.
    async fn edit_user(
        &mut self,
        id: u64,
        payload: EditUserPayload,
    ) -> crate::Result<(User, User)> {
        let mut user = get_pool()
            .fetch_user_by_id(id)
            .await?
            .ok_or_not_found("user", "user not found")?;
        let old = user.clone();

        user.username = payload.username.unwrap_or(user.username);
        user.display_name = payload
            .display_name
            .into_option_or_if_absent(user.display_name);
        user.avatar = payload.avatar.into_option_or_if_absent(user.avatar);
        user.banner = payload.banner.into_option_or_if_absent(user.banner);
        user.bio = payload.bio.into_option_or_if_absent(user.bio);

        sqlx::query!(
            r#"UPDATE users
            SET
                username = $1, display_name = $2,
                avatar = $3, banner = $4, bio = $5
            WHERE
                id = $6
            "#,
            user.username,
            user.display_name,
            user.avatar,
            user.banner,
            user.bio,
            id as i64,
        )
        .execute(self.transaction())
        .await?;

        Ok((old, user))
    }

    /// Deletes a user from the database.
    ///
    /// This method uses transactions, on the event of an ``Err`` the transaction must be properly
    /// rolled back, and the transaction must be committed to save the changes.
    ///
    /// # Errors
    /// * If an error occurs with deleting the user.
    async fn delete_user(&mut self, id: u64) -> sqlx::Result<()> {
        sqlx::query!("DELETE FROM users WHERE id = $1", id as i64)
            .execute(self.transaction())
            .await?;

        Ok(())
    }

    /// Fetches the IDs of all observable users of the user with the given ID. This returns an
    /// iterator.
    ///
    /// # Errors
    /// * If an error occurs with fetching the observable users.
    async fn fetch_observable_user_ids_for_user(&self, user_id: u64) -> crate::Result<Vec<u64>> {
        let user_ids = sqlx::query!(
            r#"SELECT DISTINCT
                id AS "id!"
            FROM members
            WHERE
                guild_id IN (SELECT guild_id FROM members WHERE id = $1)
            UNION SELECT
                target_id AS "id!"
            FROM
                relationships
            WHERE
                user_id = $1
            UNION SELECT $1 AS "id!"
            "#,
            user_id as i64,
        )
        .fetch_all(self.executor())
        .await?
        .into_iter()
        .map(|r| r.id as u64)
        .collect();

        Ok(user_ids)
    }

    /// Asserts that the user with the given ID has not blocked the user with the given ID. This
    /// returns an error if the is blocked by the user.
    ///
    /// This returns the fetched relationship type used during assertion as a side effect.
    ///
    /// # Errors
    /// * If the user cannot observe the user.
    async fn assert_user_is_not_blocked_by(
        &self,
        user_id: u64,
        target_id: u64,
    ) -> crate::Result<Option<RelationshipType>> {
        let relationship = self.fetch_relationship_type(target_id, user_id).await?;
        if let Some(relationship) = relationship
            && relationship == RelationshipType::Blocked
        {
            return Err(Error::BlockedByUser {
                target_id,
                message: "This user has blocked you, so you cannot interact with them.".to_string(),
            });
        }

        Ok(relationship)
    }

    privacy_configuration_method! {
        /// Fetches the DM privacy configuration for the user with the given ID.
        fetch_dm_privacy_configuration, "dm_privacy"
    }

    privacy_configuration_method! {
        /// Fetches the group DM privacy configuration for the user with the given ID.
        fetch_group_dm_privacy_configuration, "group_dm_privacy"
    }

    privacy_configuration_method! {
        /// Fetches the friend request privacy configuration for the user with the given ID.
        fetch_friend_request_privacy_configuration, "friend_request_privacy"
    }

    /// Asserts that the user with the given ID can interact with the user with the given ID based
    /// on the given privacy configuration.
    ///
    /// # Errors
    /// * If the user cannot interact with the user.
    async fn assert_user_can_interact_with(
        &self,
        user_id: u64,
        target_id: u64,
        interaction: UserInteractionType,
    ) -> crate::Result<()> {
        if user_id == target_id {
            return Err(Error::CannotActOnSelf {
                message: "You cannot act on yourself.".to_string(),
            });
        }

        let relationship = self
            .assert_user_is_not_blocked_by(user_id, target_id)
            .await?;
        let privacy = match interaction {
            UserInteractionType::Dm => self.fetch_dm_privacy_configuration(user_id).await?,
            UserInteractionType::GroupDm => {
                self.fetch_group_dm_privacy_configuration(user_id).await?
            }
            UserInteractionType::FriendRequest => {
                self.fetch_friend_request_privacy_configuration(user_id)
                    .await?
            }
        };

        if privacy.contains(PrivacyConfiguration::EVERYONE)
            // Friends
            || interaction != UserInteractionType::FriendRequest
                && privacy.contains(PrivacyConfiguration::FRIENDS)
                && relationship == Some(RelationshipType::Friend)

            // Mutual friends
            || privacy.contains(PrivacyConfiguration::MUTUAL_FRIENDS)
                && sqlx::query!(
                    r#"SELECT EXISTS(
                        SELECT 1 FROM relationships
                        WHERE user_id = $1 AND target_id IN (
                            SELECT target_id FROM relationships
                            WHERE user_id = $2 AND type = 'friend'
                        ) AND type = 'friend'
                    ) AS "exists!""#,
                    user_id as i64,
                    target_id as i64,
                )
                .fetch_one(self.executor())
                .await?
                .exists

            // Guild members
            || privacy.contains(PrivacyConfiguration::GUILD_MEMBERS)
                && sqlx::query!(
                    r#"SELECT EXISTS(
                        SELECT 1 FROM members
                        WHERE id = $1 AND guild_id IN (
                            SELECT guild_id FROM members WHERE id = $2
                        )
                    ) AS "exists!""#,
                    target_id as i64,
                    user_id as i64,
                )
                .fetch_one(self.executor())
                .await?
                .exists
        {
            Ok(())
        } else {
            Err(Error::UserInteractionDisallowed {
                interaction_type: interaction,
                target_id,
                message: format!(
                    "The user you are trying to {} with has privacy settings that prevent you from doing so.",
                    interaction.as_verb(),
                )
            })
        }
    }

    /// Fetches the relationship between two users.
    ///
    /// # Errors
    /// * If an error occurs with fetching the relationship.
    async fn fetch_relationship(
        &self,
        user_id: u64,
        target_id: u64,
    ) -> sqlx::Result<Option<Relationship>> {
        let relationship = query_relationships!(
            "user_id = $1 AND target_id = $2",
            user_id as i64,
            target_id as i64
        )
        .fetch_optional(self.executor())
        .await?
        .map(Relationship::from_db_relationship);

        Ok(relationship)
    }

    /// Fetches the relationship type between two users. This is used internally since it is more
    /// efficient than ``fetch_relationship``.
    ///
    /// # Errors
    /// * If an error occurs with fetching the relationship.
    async fn fetch_relationship_type(
        &self,
        user_id: u64,
        target_id: u64,
    ) -> sqlx::Result<Option<RelationshipType>> {
        struct WrappedDbRelationshipType {
            kind: DbRelationshipType,
        }

        let relationship = sqlx::query_as!(
            WrappedDbRelationshipType,
            r#"SELECT type AS "kind: _" FROM relationships WHERE user_id = $1 AND target_id = $2"#,
            user_id as i64,
            target_id as i64,
        )
        .fetch_optional(self.executor())
        .await?
        .map(|r| r.kind.into());

        Ok(relationship)
    }

    /// Fetches all relationships for the given user.
    ///
    /// # Errors
    /// * If an error occurs with fetching the relationships.
    async fn fetch_relationships(&self, user_id: u64) -> sqlx::Result<Vec<Relationship>> {
        let relationships = query_relationships!("user_id = $1", user_id as i64)
            .fetch_all(self.executor())
            .await?
            .into_iter()
            .map(Relationship::from_db_relationship)
            .collect::<Vec<_>>();

        Ok(relationships)
    }

    /// Registers a one-way relationship between two users. This is used internally.
    async fn register_one_way_relationship(
        &mut self,
        user_id: u64,
        target_id: u64,
        kind: Option<DbRelationshipType>,
    ) -> sqlx::Result<Option<Relationship>> {
        let Some(kind) = kind else {
            return Ok(query_relationships!(
                "user_id = $1 AND target_id = $2",
                user_id as i64,
                target_id as i64
            )
            .fetch_optional(get_pool())
            .await?
            .map(Relationship::from_db_relationship));
        };

        let db_relationship = sqlx::query_as!(
            DbRelationship,
            r#"WITH updated AS (
                INSERT INTO relationships
                    (user_id, target_id, type)
                VALUES
                    ($1, $2, $3)
                ON CONFLICT (user_id, target_id)
                DO UPDATE SET type = $3
                RETURNING target_id, type
            )
            SELECT
                u.id AS target_id,
                u.username AS username,
                u.display_name AS display_name,
                u.avatar AS avatar,
                u.banner AS banner,
                u.bio AS bio,
                u.flags AS flags,
                updated.type AS "kind: _"
            FROM
                updated
            INNER JOIN
                users AS u ON u.id = updated.target_id
            "#,
            user_id as i64,
            target_id as i64,
            kind as _,
        )
        .fetch_one(self.transaction())
        .await?;

        Ok(Some(Relationship::from_db_relationship(db_relationship)))
    }

    /// Creates a relationship between two users, and updates it if it already exists.
    /// Returns (user_to_target, target_to_user?).
    ///
    /// # Note
    /// This method uses transactions, on the event of an ``Err`` the transaction must be properly
    /// rolled back, and the transaction must be committed to save the changes.
    ///
    /// # Errors
    /// * If the relationship is trying to be created with a user that doesn't exist.
    /// * If an error occurs with creating the relationship.
    async fn create_relationship(
        &mut self,
        user_id: u64,
        target_id: u64,
        kind: RelationshipType,
    ) -> crate::Result<(Relationship, Option<Relationship>)> {
        let (user_kind, target_kind) = match kind {
            RelationshipType::Friend => {
                (DbRelationshipType::Friend, Some(DbRelationshipType::Friend))
            }
            RelationshipType::IncomingRequest => (
                DbRelationshipType::Incoming,
                Some(DbRelationshipType::Outgoing),
            ),
            RelationshipType::OutgoingRequest => (
                DbRelationshipType::Outgoing,
                Some(DbRelationshipType::Incoming),
            ),
            RelationshipType::Blocked => (DbRelationshipType::Blocked, None),
        };

        let relationship = self
            .register_one_way_relationship(user_id, target_id, Some(user_kind))
            .await?
            // TODO: Should this really panic?
            .expect("relationship should have been upserted");
        let external_relationship = self
            .register_one_way_relationship(target_id, user_id, target_kind)
            .await?;

        Ok((relationship, external_relationship))
    }

    /// Deletes a relationship between two users. Returns the number of rows affected.
    ///
    /// # Note
    /// This method uses transactions, on the event of an ``Err`` the transaction must be properly
    /// rolled back, and the transaction must be committed to save the changes.
    ///
    /// # Errors
    /// * If an error occurs with deleting the relationship.
    /// * If the relationship doesn't exist.
    async fn delete_relationship(&mut self, user_id: u64, target_id: u64) -> crate::Result<u64> {
        Ok(sqlx::query!(
            r#"DELETE FROM
                relationships
            WHERE
                user_id = $1 AND target_id = $2
            OR
                target_id = $1 AND user_id = $2 AND type != 'blocked'
            "#,
            user_id as i64,
            target_id as i64,
        )
        .execute(self.transaction())
        .await?
        .rows_affected())
    }

    async fn fetch_user_settings(&self, user_id: u64) -> crate::Result<Settings> {
        let settings = sqlx::query!("SELECT settings FROM users WHERE id = $1", user_id as i64)
            .fetch_one(self.executor())
            .await?
            .settings;

        Ok(Settings::from_bits_truncate(settings))
    }

    async fn update_user_settings(
        &mut self,
        user_id: u64,
        settings: Settings,
    ) -> crate::Result<()> {
        sqlx::query!(
            "UPDATE users SET settings = $1 WHERE id = $2",
            settings.bits(),
            user_id as i64
        )
        .execute(self.transaction())
        .await?;

        Ok(())
    }

    async fn fetch_notification_settings(
        &self,
        user_id: u64,
    ) -> crate::Result<HashMap<u64, NotificationFlags>> {
        Ok(sqlx::query!(
            "SELECT target_id, notif_flags FROM notification_settings WHERE user_id = $1",
            user_id as i64
        )
        .fetch_all(self.executor())
        .await?
        .into_iter()
        .map(|r| {
            (
                r.target_id as u64,
                NotificationFlags::from_bits_truncate(r.notif_flags),
            )
        })
        .collect())
    }

    async fn fetch_notification_settings_in_target(
        &self,
        user_id: u64,
        target_id: u64,
    ) -> crate::Result<Option<NotificationFlags>> {
        Ok(sqlx::query!(
            "SELECT notif_flags FROM notification_settings WHERE user_id = $1 AND target_id = $2",
            user_id as i64,
            target_id as i64
        )
        .fetch_optional(self.executor())
        .await?
        .map(|r| NotificationFlags::from_bits_truncate(r.notif_flags)))
    }

    async fn update_notification_settings(
        &mut self,
        user_id: u64,
        target_id: u64,
        flags: NotificationFlags,
    ) -> crate::Result<()> {
        sqlx::query!(
            r#"INSERT INTO 
                notification_settings 
            VALUES 
                ($1, $2, $3) 
            ON CONFLICT 
                (user_id, target_id) 
            DO UPDATE SET 
                notif_flags = $3
            "#,
            user_id as i64,
            target_id as i64,
            flags.bits()
        )
        .execute(self.transaction())
        .await?;

        Ok(())
    }

    async fn remove_notification_settings(
        &mut self,
        user_id: u64,
        target_id: u64,
    ) -> crate::Result<()> {
        sqlx::query!(
            "DELETE FROM notification_settings WHERE user_id = $1 AND target_id = $2",
            user_id as i64,
            target_id as i64
        )
        .execute(self.transaction())
        .await?;

        Ok(())
    }

    async fn can_push(&self, user_id: u64, _target_id: Option<u64>) -> crate::Result<bool> {
        let enabled = self
            .fetch_user_settings(user_id)
            .await?
            .contains(Settings::NOTIFICATIONS);
        // TODO: Check override and target.

        Ok(enabled)
    }

    /// Registers a new bot account with the given payload.
    ///
    /// # Note
    /// This method uses transactions, on the event of an ``Err`` the transaction must be properly
    /// rolled back, and the transaction must be committed to save the changes.
    ///
    /// # Errors
    /// * If an error occurs with registering the bot.
    async fn create_bot(
        &mut self,
        id: u64,
        owner_id: u64,
        qualified_name: impl AsRef<str> + Send,
        display_name: Option<impl AsRef<str> + Send>,
        flags: BotFlags,
    ) -> crate::Result<Bot> {
        let user = sqlx::query!(
            r"INSERT INTO users (id, username, display_name, flags)
            VALUES ($1, $2, $3, $4)
            RETURNING *",
            id as i64,
            qualified_name.as_ref().trim(),
            display_name.as_ref().map(|s| s.as_ref().trim()),
            UserFlags::BOT.bits() as i32,
        )
        .fetch_one(self.transaction())
        .await?;

        sqlx::query!(
            "INSERT INTO bots (user_id, owner_id, flags) VALUES ($1, $2, $3)",
            id as i64,
            owner_id as i64,
            flags.bits() as i32,
        )
        .execute(self.transaction())
        .await?;

        Ok(Bot {
            user: construct_user!(user),
            owner_id,
            default_permissions: Permissions::empty(),
            flags,
        })
    }

    /// Fetches a bot from the database with the given ID.
    async fn fetch_bot(&self, id: u64) -> crate::Result<Option<Bot>> {
        let bot = query_bots!("u.id = $1", id as i64)
            .fetch_optional(self.executor())
            .await?
            .map(|b| construct_bot!(b));

        Ok(bot)
    }

    /// Fetches all bots from the database owned by the user with the given ID.
    async fn fetch_all_bots_by_user(&self, user_id: u64) -> crate::Result<Vec<Bot>> {
        let bots = query_bots!("b.owner_id = $1", user_id as i64)
            .fetch_all(self.executor())
            .await?
            .into_iter()
            .map(|b| construct_bot!(b))
            .collect();

        Ok(bots)
    }

    /// Modifies a bot in the database with the given payload. Validation must be done before
    /// calling this method.
    ///
    /// # Note
    /// This method uses transactions, on the event of an ``Err`` the transaction must be properly
    /// rolled back, and the transaction must be committed to save the changes.
    async fn edit_bot(&mut self, user: User, payload: EditBotPayload) -> crate::Result<Bot> {
        let bot = sqlx::query!("SELECT * FROM bots WHERE user_id = $1", user.id as i64)
            .fetch_one(self.transaction())
            .await?;

        let mut flags = BotFlags::from_bits_truncate(bot.flags as _);
        macro_rules! toggle {
            ($($field:ident => $flag:ident),+) => {{
                $(
                    if let Some(toggle) = payload.$field {
                        flags.set(BotFlags::$flag, toggle);
                    }
                )+
            }};
        }

        toggle! {
            public => PUBLIC,
            global_enabled => GLOBAL_ENABLED,
            group_dm_enabled => GROUP_DM_ENABLED,
            guild_enabled => GUILD_ENABLED
        }

        let permissions = payload
            .default_permissions
            .map_or(bot.default_permissions, |p| p.bits());

        sqlx::query!(
            "UPDATE bots SET flags = $1, default_permissions = $2 WHERE user_id = $3",
            flags.bits() as i32,
            permissions,
            user.id as i64
        )
        .execute(self.transaction())
        .await?;

        Ok(Bot {
            user,
            owner_id: bot.owner_id as _,
            default_permissions: Permissions::from_bits_truncate(permissions),
            flags,
        })
    }

    /// Deletes a bot from the database.
    ///
    /// # Note
    /// This method uses transactions, on the event of an ``Err`` the transaction must be properly
    /// rolled back, and the transaction must be committed to save the changes.
    async fn delete_bot(&mut self, id: u64) -> crate::Result<()> {
        sqlx::query!("DELETE FROM bots WHERE user_id = $1", id as i64)
            .execute(self.transaction())
            .await?;

        Ok(())
    }
}

impl<'t, T> UserDbExt<'t> for T where T: DbExt<'t> {}
