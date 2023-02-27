use bincode::{Decode, Encode};
use deadpool_redis::redis::{ErrorKind, FromRedisValue, RedisError, ToRedisArgs, Value};

const CONFIG: bincode::config::Configuration = bincode::config::standard();

pub struct BincodeType<T>(pub T);

impl<T: Encode> ToRedisArgs for BincodeType<T> {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + deadpool_redis::redis::RedisWrite,
    {
        let v = bincode::encode_to_vec(&self.0, CONFIG).expect("failed to serialize item");

        out.write_arg(&v);
    }
}

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
