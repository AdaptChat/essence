use crate::db::guild::construct_partial_guild;
use crate::{
    Error, NotFoundExt, cache,
    db::{DbExt, GuildDbExt, MemberDbExt},
    http::invite::CreateInvitePayload,
    models::{Member, invite::Invite},
};

macro_rules! construct_invite {
    ($data:ident, $guild:expr_2021) => {{
        Invite {
            code: $data.code,
            guild_id: $data.guild_id as _,
            guild: $guild,
            created_at: Some($data.created_at),
            inviter_id: Some($data.inviter_id as _),
            max_age: $data.max_age as _,
            max_uses: $data.max_uses as _,
            uses: $data.uses as _,
        }
    }};
}

#[async_trait::async_trait]
pub trait InviteDbExt<'t>: DbExt<'t> {
    /// Resolves the partial guild associated with the given vanity invite code, and then promotes
    /// it to an [`Invite`].
    ///
    /// # Errors
    /// * If an error occurs with fetching the vanity invite.
    async fn fetch_vanity_invite(&self, vanity_code: &str) -> sqlx::Result<Option<Invite>> {
        let partial_guild = sqlx::query!(
            r#"SELECT
                *,
                (SELECT COUNT(*) FROM members WHERE guild_id = guilds.id) AS "member_count!"
            FROM
                guilds
            WHERE
                vanity_url = $1
            "#,
            vanity_code,
        )
        .fetch_optional(self.executor())
        .await
        .map(|r| r.map(|p| construct_partial_guild!(p)))?;

        Ok(partial_guild.map(|p| Invite {
            code: vanity_code.to_string(),
            guild_id: p.id,
            guild: Some(p),
            created_at: None,
            inviter_id: None,
            max_age: 0,
            max_uses: 0,
            uses: 0,
        }))
    }

    /// Fetches an invite from the database with the given code. Returns `None` if the invite is not
    /// found. Since this is fetching a single invite, this will include guild information.
    ///
    /// # Errors
    /// * If an error occurs with fetching the invite.
    /// * If an error occurs with fetching the guild.
    async fn fetch_invite(&self, code: impl AsRef<str> + Send) -> sqlx::Result<Option<Invite>> {
        if let Some(invite) = self.fetch_vanity_invite(code.as_ref()).await? {
            return Ok(Some(invite));
        };
        let Some(i) = sqlx::query!(
            r#"SELECT * FROM invites
            WHERE
                code = $1
                AND (max_age = 0 OR created_at + max_age * interval '1 second' > NOW())
            "#,
            code.as_ref(),
        )
        .fetch_optional(self.executor())
        .await?
        else {
            return Ok(None);
        };

        Ok(Some(construct_invite!(
            i,
            self.fetch_partial_guild(i.guild_id as u64).await?
        )))
    }

    /// Fetches all invites within a given guild.
    ///
    /// # Errors
    /// * If the guild is not found.
    /// * If an error occurs with fetching the invites.
    async fn fetch_all_invites_in_guild(&self, guild_id: u64) -> crate::Result<Vec<Invite>> {
        let invites = sqlx::query!(
            r#"SELECT * FROM invites
            WHERE
                guild_id = $1
                AND (max_age = 0 OR created_at + max_age * interval '1 second' > NOW())
            "#,
            guild_id as i64,
        )
        .fetch_all(self.executor())
        .await?
        .into_iter()
        .map(|i| construct_invite!(i, None))
        .collect();

        Ok(invites)
    }

    /// Uses an invite and increments the uses counter.
    ///
    /// # Note
    /// This method uses transactions, on the event of an ``Err`` the transaction must be properly
    /// rolled back, and the transaction must be committed to save the changes.
    ///
    /// # Errors
    /// * If no invite is found with the given invite code.
    /// * If an error occurs with creating the invite.
    #[allow(clippy::default_trait_access)]
    async fn use_invite(
        &mut self,
        user_id: u64,
        code: impl AsRef<str> + Send,
    ) -> crate::Result<(Invite, Option<Member>)> {
        let code = code.as_ref();
        let invite = sqlx::query!(
            r#"UPDATE invites
            SET uses = uses + 1
            WHERE
                code = $1
                AND (max_age = 0 OR created_at + max_age * interval '1 second' > NOW())
            RETURNING *
            "#,
            code,
        )
        .fetch_optional(self.transaction())
        .await?
        .ok_or_not_found("invite", format!("No invite with code {code} found"))?;

        let invite = construct_invite!(invite, None);
        if invite.max_uses != 0 && invite.uses >= invite.max_uses {
            self.delete_invite(code).await?;
        }

        let guild_id = invite.guild_id;

        if cache::is_banned(guild_id, user_id).await? {
            return Err(Error::BannedFromGuild {
                guild_id,
                reason: None,
                message: format!("You are banned from guild {guild_id}"),
            });
        }

        Ok((
            invite,
            self.create_member(guild_id, user_id, Default::default())
                .await?,
        ))
    }

    /// Creates an invite for the given guild.
    ///
    /// # Note
    /// This method uses transactions, on the event of an ``Err`` the transaction must be properly
    /// rolled back, and the transaction must be committed to save the changes.
    ///
    /// # Errors
    /// * If the guild is not found.
    /// * If an error occurs with creating the invite.
    async fn create_invite(
        &mut self,
        guild_id: u64,
        inviter_id: u64,
        code: String,
        payload: CreateInvitePayload,
    ) -> crate::Result<Invite> {
        let created_at = sqlx::query!(
            r#"INSERT INTO invites
                (code, inviter_id, guild_id, max_uses, max_age)
            VALUES
                ($1, $2, $3, $4, $5)
            ON CONFLICT (code) DO NOTHING
            RETURNING created_at
            "#,
            code,
            inviter_id as i64,
            guild_id as i64,
            payload.max_uses as i32,
            payload.max_age as i32,
        )
        .fetch_optional(self.transaction())
        .await?
        .ok_or_else(|| Error::InternalError {
            what: Some("invite_code".to_string()),
            message: "Conflict was encountered when creating invite".to_string(),
            debug: None,
        })?
        .created_at;

        cache::update_invite(&code, guild_id).await?;
        Ok(Invite {
            code,
            inviter_id: Some(inviter_id),
            guild: None,
            guild_id,
            created_at: Some(created_at),
            uses: 0,
            max_uses: payload.max_uses,
            max_age: payload.max_age,
        })
    }

    /// Deletes (revokes) the invite with the given code.
    ///
    /// # Note
    /// This method uses transactions, on the event of an ``Err`` the transaction must be properly
    /// rolled back, and the transaction must be committed to save the changes.
    ///
    /// # Errors
    /// * If the guild is not found.
    /// * If an error occurs with creating the invite.
    async fn delete_invite(&mut self, code: impl AsRef<str> + Send) -> crate::Result<()> {
        sqlx::query!(r#"DELETE FROM invites WHERE code = $1"#, code.as_ref())
            .execute(self.transaction())
            .await?;

        cache::invalidate_invite(code.as_ref()).await?;
        Ok(())
    }

    /// Deletes all invites within a given guild. This does not include the vanity invite.
    ///
    /// # Note
    /// This method uses transactions, on the event of an ``Err`` the transaction must be properly
    /// rolled back, and the transaction must be committed to save the changes.
    ///
    /// # Errors
    /// * If the guild is not found.
    /// * If an error occurs with creating the invite.
    async fn delete_all_invites_in_guild(&mut self, guild_id: u64) -> crate::Result<()> {
        let codes = sqlx::query!(
            r#"DELETE FROM invites WHERE guild_id = $1 RETURNING code"#,
            guild_id as i64,
        )
        .fetch_all(self.transaction())
        .await?;

        for record in codes {
            cache::invalidate_invite(&record.code).await?;
        }
        Ok(())
    }
}

impl<'t, T> InviteDbExt<'t> for T where T: DbExt<'t> {}
