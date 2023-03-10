use super::DbExt;
use crate::{
    db::get_pool,
    error::UserInteractionType,
    http::user::EditUserPayload,
    models::{ClientUser, PrivacyConfiguration, Relationship, RelationshipType, User, UserFlags},
    Error, NotFoundExt,
};

macro_rules! construct_user {
    ($data:ident) => {{
        User {
            id: $data.id as _,
            username: $data.username,
            discriminator: $data.discriminator as _,
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
    ($self:ident, $query:literal, $($arg:expr),* $(,)?) => {{
        let result = sqlx::query!($query, $($arg),*)
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
            });

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
                u.discriminator AS discriminator,
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
    pub discriminator: i16,
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

    /// Fetches a user from the database with the given username and discriminator.
    ///
    /// # Errors
    /// * If an error occurs with fetching the user. If the user is not found, `Ok(None)` is
    /// returned.
    async fn fetch_user_by_tag(
        &self,
        username: &str,
        discriminator: u16,
    ) -> sqlx::Result<Option<User>> {
        fetch_user!(
            self,
            "SELECT * FROM users WHERE username = $1 AND discriminator = $2",
            username,
            discriminator as i16,
        )
    }

    /// Fetches the client user from the database.
    ///
    /// # Errors
    /// * If an error occurs with fetching the client user.
    async fn fetch_client_user_by_id(&self, id: u64) -> sqlx::Result<Option<ClientUser>> {
        fetch_client_user!(self, "SELECT * FROM users WHERE id = $1", id as i64)
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
        fetch_client_user!(self, "SELECT * FROM users WHERE email = $1", email.as_ref())
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
        email: impl AsRef<str> + Send,
        password: impl AsRef<str> + Send,
    ) -> crate::Result<()> {
        let password = password.as_ref();
        let hashed = crate::auth::hash_password(password).await?;

        let discriminator = sqlx::query!(
            "INSERT INTO
                users (id, username, email, password)
            VALUES
                ($1, $2, $3, $4)
            RETURNING
                discriminator",
            id as i64,
            username.as_ref().trim(),
            email.as_ref().trim(),
            hashed,
        )
        .fetch_optional(self.transaction())
        .await?;

        if discriminator.is_none() {
            return Err(Error::AlreadyTaken {
                what: "username".to_string(),
                message: "Username is already taken".to_string(),
            });
        }

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

        if let Some(username) = payload.username {
            let discriminator = if sqlx::query!(
                "SELECT discriminator FROM users WHERE id != $1 AND username = $2 AND discriminator = $3",
                id as i64,
                username,
                user.discriminator as i16,
            )
            .fetch_optional(get_pool())
            .await?
            .is_none()
            {
                user.discriminator as i16
            } else {
                let discriminator =
                    sqlx::query!("SELECT generate_discriminator($1) AS out", username)
                        .fetch_one(get_pool())
                        .await?
                        .out;

                discriminator.ok_or_else(|| Error::AlreadyTaken {
                    what: "username".to_string(),
                    message: "Username is already taken".to_string(),
                })?
            };

            sqlx::query!(
                "UPDATE users SET username = $1, discriminator = $2 WHERE id = $3",
                username,
                discriminator,
                id as i64,
            )
            .execute(self.transaction())
            .await?;

            user.username = username;
            user.discriminator = discriminator as u16;
        }

        user.avatar = payload.avatar.into_option_or_if_absent(user.avatar);
        user.banner = payload.banner.into_option_or_if_absent(user.banner);
        user.bio = payload.bio.into_option_or_if_absent(user.bio);

        sqlx::query!(
            r#"UPDATE users SET avatar = $1, banner = $2, bio = $3 WHERE id = $4"#,
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
            return Ok(
                query_relationships!("user_id = $1 AND target_id = $2", user_id as i64, target_id as i64)
                    .fetch_optional(get_pool())
                    .await?
                    .map(Relationship::from_db_relationship)
            )
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
                u.discriminator AS discriminator,
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
}

impl<'t, T> UserDbExt<'t> for T where T: DbExt<'t> {}
