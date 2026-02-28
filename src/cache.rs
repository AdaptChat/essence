use crate::{
    bincode_impl::BincodeType,
    error::{ErrIntoExt, Result},
    models::{ChannelType, Permissions, User, UserFlags},
};
use deadpool_redis::{Config, Connection, Pool, Runtime, redis::AsyncCommands};
use std::sync::OnceLock;

static POOL: OnceLock<Pool> = OnceLock::new();

pub trait AsRefThreadSafe<T: ?Sized> = AsRef<T> + Send + Sync;

#[derive(Clone, bincode::Encode, bincode::Decode)]
pub struct ChannelInspection {
    pub guild_id: Option<u64>,
    pub owner_id: Option<u64>,
    pub channel_type: ChannelType,
}

pub(crate) fn connect(url: &str) {
    POOL.set(
        Config::from_url(url)
            .create_pool(Some(Runtime::Tokio1))
            .unwrap(),
    )
    .unwrap_or_else(|_| panic!("Failed to set `POOL`"));
}

async fn get_con() -> Result<Connection> {
    unsafe { Ok(POOL.get().unwrap_unchecked().get().await?) }
}

pub async fn user_info_for_token(
    token: impl AsRefThreadSafe<str>,
) -> Result<Option<(u64, UserFlags)>> {
    Ok(get_con()
        .await?
        .hget::<_, _, Option<BincodeType<(u64, UserFlags)>>>("essence-tokens", token.as_ref())
        .await?
        .map(|v| v.0))
}

pub async fn cache_token(
    token: impl AsRefThreadSafe<str>,
    user_id: u64,
    flags: UserFlags,
) -> Result<()> {
    let () = get_con()
        .await?
        .hset(
            "essence-tokens",
            token.as_ref(),
            BincodeType((user_id, flags)),
        )
        .await?;

    Ok(())
}

pub async fn invalidate_token(token: String) -> Result<()> {
    get_con()
        .await?
        .hdel("essence-tokens", token)
        .await
        .err_into()
}

pub async fn invalidate_tokens_for(user_id: u64) -> Result<()> {
    let mut con = get_con().await?;

    let tokens = con
        .hgetall::<_, Vec<(String, BincodeType<(u64, UserFlags)>)>>("essence-tokens")
        .await?
        .into_iter()
        .filter_map(|(token, x)| {
            let user = x.0.0;

            if user == user_id { Some(token) } else { None }
        })
        .collect::<Vec<String>>();

    if !tokens.is_empty() {
        let () = con.hdel("essence-tokens", tokens).await?;
    }
    Ok(())
}

pub async fn update_user(user: User) -> Result<()> {
    get_con()
        .await?
        .hset("essence-users", user.id, BincodeType(user))
        .await
        .err_into()
}

pub async fn user(user_id: u64) -> Result<Option<User>> {
    Ok(get_con()
        .await?
        .hget::<_, _, Option<BincodeType<User>>>("essence-users", user_id)
        .await?
        .map(|u| u.0))
}

pub async fn remove_user(user_id: u64) -> Result<()> {
    get_con()
        .await?
        .hdel("essence-users", user_id)
        .await
        .err_into()
}

pub async fn update_channel(channel_id: u64, inspection: ChannelInspection) -> Result<()> {
    get_con()
        .await?
        .hset("essence-channels", channel_id, BincodeType(inspection))
        .await
        .err_into()
}

pub async fn inspection_for_channel(channel_id: u64) -> Result<Option<ChannelInspection>> {
    Ok(get_con()
        .await?
        .hget::<_, _, Option<BincodeType<ChannelInspection>>>("essence-channels", channel_id)
        .await?
        .map(|v| v.0))
}

pub async fn remove_channel(channel_id: u64) -> Result<()> {
    get_con()
        .await?
        .hdel("essence-channels", channel_id)
        .await
        .err_into()
}

pub async fn remove_guild(guild_id: u64) -> Result<()> {
    let mut con = get_con().await?;

    let keys = con
        .keys::<_, Vec<String>>(format!("essence-{guild_id}-*"))
        .await?;
    let () = con.del(keys).await?;

    con.srem("essence-guilds", guild_id).await.err_into()
}

pub async fn insert_guild(guild_id: u64) -> Result<()> {
    get_con()
        .await?
        .sadd("essence-guilds", guild_id)
        .await
        .err_into()
}

pub async fn insert_guilds(guild_ids: impl AsRefThreadSafe<[u64]>) -> Result<()> {
    get_con()
        .await?
        .sadd("essence-guilds", guild_ids.as_ref())
        .await
        .err_into()
}

pub async fn guild_exist(guild_id: u64) -> Result<Option<()>> {
    Ok(get_con()
        .await?
        .sismember::<_, _, bool>("essence-guilds", guild_id)
        .await?
        .then_some(()))
}

pub async fn is_member_of_guild(guild_id: u64, user_id: u64) -> Result<Option<()>> {
    Ok(get_con()
        .await?
        .sismember::<_, _, bool>(format!("essence-{guild_id}-members"), user_id)
        .await?
        .then_some(()))
}

pub async fn remove_member_from_guild(guild_id: u64, user_id: u64) -> Result<()> {
    delete_permissions_for_user(guild_id, user_id).await.ok();
    get_con()
        .await?
        .srem(format!("essence-{guild_id}-members"), user_id)
        .await
        .err_into()
}

pub async fn update_member_of_guild(guild_id: u64, user_id: u64) -> Result<()> {
    get_con()
        .await?
        .sadd(format!("essence-{guild_id}-members"), user_id)
        .await
        .err_into()
}

pub async fn update_members_of_guild(
    guild_id: u64,
    user_ids: impl AsRefThreadSafe<[u64]>,
) -> Result<()> {
    get_con()
        .await?
        .sadd(format!("essence-{guild_id}-members"), user_ids.as_ref())
        .await
        .err_into()
}

pub async fn update_owner_of_guild(guild_id: u64, user_id: u64) -> Result<()> {
    get_con()
        .await?
        .set(format!("essence-{guild_id}-owner"), user_id)
        .await
        .err_into()
}

pub async fn owner_of_guild(guild_id: u64) -> Result<Option<u64>> {
    get_con()
        .await?
        .get(format!("essence-{guild_id}-owner"))
        .await
        .err_into()
}

pub async fn update_permissions_for(
    guild_id: u64,
    user_id: u64,
    channel_id: Option<u64>,
    permissions: Permissions,
) -> Result<()> {
    get_con()
        .await?
        .hset(
            format!("essence-{guild_id}-{user_id}-perm"),
            channel_id.unwrap_or(0),
            permissions.bits(),
        )
        .await
        .err_into()
}

pub async fn permissions_for(
    guild_id: u64,
    user_id: u64,
    channel_id: Option<u64>,
) -> Result<Option<Permissions>> {
    Ok(get_con()
        .await?
        .hget::<_, _, Option<i64>>(
            format!("essence-{guild_id}-{user_id}-perm"),
            channel_id.unwrap_or(0),
        )
        .await?
        .map(Permissions::from_bits_truncate))
}

pub async fn delete_permissions_for_user(guild_id: u64, user_id: u64) -> Result<()> {
    get_con()
        .await?
        .del(format!("essence-{guild_id}-{user_id}-perm"))
        .await
        .err_into()
}

pub async fn delete_permissions_for_channel(guild_id: u64, channel_id: u64) -> Result<()> {
    let mut con = get_con().await?;
    let keys = con
        .keys::<_, Vec<String>>(format!("essence-{guild_id}-*-perm"))
        .await?;

    con.hdel(keys, channel_id).await.err_into()
}

pub async fn delete_permissions_for_user_in_channel(
    guild_id: u64,
    user_id: u64,
    channel_id: Option<u64>,
) -> Result<()> {
    get_con()
        .await?
        .hdel(
            format!("essence-{guild_id}-{user_id}-perm"),
            channel_id.unwrap_or(0),
        )
        .await
        .err_into()
}

pub async fn clear_member_permissions(guild_id: u64) -> Result<()> {
    let mut con = get_con().await?;
    let keys = con
        .keys::<_, Vec<String>>(format!("essence-{guild_id}-*-perm"))
        .await?;

    con.del(keys).await.err_into()
}

pub async fn is_banned(guild_id: u64, user_id: u64) -> Result<bool> {
    get_con()
        .await?
        .sismember(format!("essence-{guild_id}-bans"), user_id)
        .await
        .err_into()
}

pub async fn add_ban(guild_id: u64, user_id: u64) -> Result<()> {
    get_con()
        .await?
        .sadd(format!("essence-{guild_id}-bans"), user_id)
        .await
        .err_into()
}

pub async fn remove_ban(guild_id: u64, user_id: u64) -> Result<()> {
    get_con()
        .await?
        .srem(format!("essence-{guild_id}-bans"), user_id)
        .await
        .err_into()
}

pub async fn store_email_verification(
    user_id: u64,
    code: &str,
    pending_email: Option<&str>,
) -> Result<()> {
    // 10 minute window to verify
    const VERIFICATION_CODE_TTL_SECS: u64 = 600;

    let value = format!("{}:{}", code, pending_email.unwrap_or(""));
    get_con()
        .await?
        .set_ex(
            format!("essence-email-verify-{user_id}"),
            value,
            VERIFICATION_CODE_TTL_SECS,
        )
        .await
        .err_into()
}

/// Returns ``Some((code, pending_email))``` if an entry exists.
pub async fn get_email_verification(user_id: u64) -> Result<Option<(String, Option<String>)>> {
    let raw: Option<String> = get_con()
        .await?
        .get(format!("essence-email-verify-{user_id}"))
        .await?;

    Ok(raw.map(|s| {
        let (code, email) = s.split_once(':').unwrap_or((&s, ""));
        (
            code.to_string(),
            if email.is_empty() {
                None
            } else {
                Some(email.to_string())
            },
        )
    }))
}

pub async fn delete_email_verification(user_id: u64) -> Result<()> {
    get_con()
        .await?
        .del(format!("essence-email-verify-{user_id}"))
        .await
        .err_into()
}

pub async fn resolve_invite_guild_id(code: &str) -> Result<Option<u64>> {
    get_con()
        .await?
        .get(format!("essence-invite-{code}"))
        .await
        .err_into()
}

pub async fn invalidate_invite(code: &str) -> Result<()> {
    let mut con = get_con().await?;
    con.del(format!("essence-invite-{code}")).await.err_into()
}

pub async fn update_invite(code: &str, guild_id: u64) -> Result<()> {
    let mut con = get_con().await?;
    con.set(format!("essence-invite-{code}"), guild_id)
        .await
        .err_into()
}
