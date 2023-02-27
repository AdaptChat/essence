use std::sync::OnceLock;

use deadpool_redis::{redis::AsyncCommands, Config, Connection, Pool, Runtime};

use crate::{
    bincode_impl::BincodeType,
    error::Result,
    models::{Permissions, User, UserFlags},
};

static POOL: OnceLock<Pool> = OnceLock::new();

type ResultOption<T> = Result<Option<T>>;

fn setup() {
    POOL.set(
        Config::from_url("redis://127.0.0.1")
            .create_pool(Some(Runtime::Tokio1))
            .unwrap(),
    )
    .unwrap_or_else(|_| panic!("Failed to set `POOL`"));
}

async fn get_con() -> Result<Connection> {
    unsafe { Ok(POOL.get().unwrap_unchecked().get().await?) }
}

async fn user_info_for_token(token: String) -> ResultOption<(u64, UserFlags)> {
    Ok(get_con()
        .await?
        .hget::<_, _, Option<BincodeType<(u64, UserFlags)>>>("essence-tokens", token)
        .await?
        .map(|v| v.0))
}

pub async fn cache_token(token: String, user_id: u64, flags: UserFlags) -> Result<()> {
    get_con()
        .await?
        .hset("essence-tokens", token, BincodeType((user_id, flags)))
        .await?;

    Ok(())
}

pub async fn invalidate_token(token: String) -> Result<()> {
    get_con().await?.hdel("essence-tokens", token).await?;

    Ok(())
}

pub async fn invalidate_tokens_for(user_id: u64) -> Result<()> {
    let mut con = get_con().await?;

    let tokens = con
        .hgetall::<_, Vec<(String, BincodeType<(u64, UserFlags)>)>>("essence-tokens")
        .await?
        .into_iter()
        .filter_map(|(token, x)| {
            let user = x.0 .0;

            if user == user_id {
                Some(token)
            } else {
                None
            }
        })
        .collect::<Vec<String>>();

    Ok(con.hdel("essence-tokens", tokens).await?)
}

pub async fn update_user(user: User) -> Result<()> {
    Ok(get_con()
        .await?
        .hset("essence-users", user.id, BincodeType(user))
        .await?)
}

pub async fn user(user_id: u64) -> Result<Option<User>> {
    Ok(get_con()
        .await?
        .hget::<_, _, Option<BincodeType<User>>>("essence-users", user_id)
        .await?
        .map(|u| u.0))
}

pub async fn remove_user(user_id: u64) -> Result<()> {
    Ok(get_con().await?.hdel("essence-users", user_id).await?)
}

pub async fn remove_guild(guild_id: u64) -> Result<()> {
    let mut con = get_con().await?;

    let keys = con.keys::<_, Vec<String>>(format!("{guild_id}-*")).await?;
    con.del(keys).await?;

    Ok(())
}

pub async fn is_member_of_guild(guild_id: u64, user_id: u64) -> ResultOption<bool> {
    Ok(get_con()
        .await?
        .sismember::<_, _, bool>(format!("{guild_id}-members"), user_id)
        .await?
        .then_some(true))
}

pub async fn remove_member_from_guild(guild_id: u64, user_id: u64) -> Result<()> {
    Ok(get_con()
        .await?
        .srem(format!("{guild_id}-members"), user_id)
        .await?)
}

pub async fn update_member_of_guild(guild_id: u64, user_id: u64) -> Result<()> {
    Ok(get_con()
        .await?
        .sadd(format!("{guild_id}-members"), user_id)
        .await?)
}

pub async fn update_members_of_guild(guild_id: u64, user_ids: impl AsRef<[u64]>) -> Result<()> {
    Ok(get_con()
        .await?
        .sadd(format!("{guild_id}-members"), user_ids.as_ref())
        .await?)
}

pub async fn update_owner_of_guild(guild_id: u64, user_id: u64) -> Result<()> {
    Ok(get_con()
        .await?
        .set(format!("{guild_id}-owner"), user_id)
        .await?)
}

pub async fn owner_of_guild(guild_id: u64) -> Result<Option<u64>> {
    Ok(get_con().await?.get(format!("{guild_id}-owner")).await?)
}

pub async fn update_permissions_for(
    guild_id: u64,
    user_id: u64,
    channel_id: Option<u64>,
    permissions: Permissions,
) -> Result<()> {
    Ok(get_con()
        .await?
        .hset(
            format!("{guild_id}-{user_id}-perm"),
            channel_id.unwrap_or(0),
            permissions.bits(),
        )
        .await?)
}

pub async fn permissions_for(
    guild_id: u64,
    user_id: u64,
    channel_id: Option<u64>,
) -> ResultOption<Permissions> {
    Ok(get_con()
        .await?
        .hget::<_, _, Option<i64>>(
            format!("{guild_id}-{user_id}-perm"),
            channel_id.unwrap_or(0),
        )
        .await?
        .map(Permissions::from_bits_truncate))
}

pub async fn delete_permissions_for_user(guild_id: u64, user_id: u64) -> Result<()> {
    Ok(get_con()
        .await?
        .del(format!("{guild_id}-{user_id}"))
        .await?)
}

pub async fn delete_permissions_for_user_in_channel(
    guild_id: u64,
    user_id: u64,
    channel_id: Option<u64>,
) -> Result<()> {
    Ok(get_con()
        .await?
        .hdel(format!("{guild_id}-{user_id}"), channel_id.unwrap_or(0))
        .await?)
}
