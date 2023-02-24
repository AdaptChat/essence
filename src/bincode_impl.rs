use bincode::Decode;
use deadpool_redis::redis::{ErrorKind, FromRedisValue, RedisError, Value};

pub struct BincodeType<T: Decode>(pub T);

impl<T: Decode> FromRedisValue for BincodeType<T> {
    fn from_redis_value(v: &Value) -> deadpool_redis::redis::RedisResult<Self> {
        match v {
            Value::Data(d) => Ok(Self(
                bincode::decode_from_slice::<T, _>(d, bincode::config::standard())
                    .map_err(|e| {
                        RedisError::from((
                            ErrorKind::TypeError,
                            "Response was of incompatable type",
                            format!("{e:?} (response was: {v:?})"),
                        ))
                    })?
                    .0,
            )),
            _ => Err(RedisError::from((
                ErrorKind::TypeError,
                "Response was of incompatable type",
                format!("(response was: {v:?})"),
            ))),
        }
    }
}
