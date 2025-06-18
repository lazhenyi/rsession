use crate::storage::SessionStore;
use deadpool_redis::redis::AsyncCommands;
use std::collections::HashMap;
use std::io::Error;
use time::Duration;

#[derive(Clone)]
pub struct RedisSessionStorage {
    pub conn: deadpool_redis::Pool,
}



impl RedisSessionStorage {
    pub fn new(pool: deadpool_redis::Pool) -> Self {
        RedisSessionStorage {
            conn: pool,
        }
    }
    async fn get_conn(&self) -> Result<deadpool_redis::Connection, Error> {
        match self.conn.get().await {
            Ok(x) => Ok(x),
            Err(x) => Err(Error::new(
                std::io::ErrorKind::Other,
                x
            ))
        }
    }
}

impl SessionStore for RedisSessionStorage {
    async fn get(&self, key: &str) -> Result<HashMap<String, String>, Error> {
        let mut conn = self.get_conn().await?;
        let value = conn.get::<&str, String>(key).await
            .map_err(|err| {
                Error::new(
                    std::io::ErrorKind::Other,
                    err
                )
            });
        value.map(|x| {
            serde_json::from_str::<HashMap<String, String>>(&x)
                .map_err(|err| {
                    Error::new(
                        std::io::ErrorKind::Other,
                        err
                    )
                })
        })?
            
    }

    async fn set(&self, key: &str, value: HashMap<String, String>) -> Result<(), Error> {
        let mut conn = self.get_conn().await?;
        conn.set(key, serde_json::to_string(&value)?).await
            .map_err(|err| {
                Error::new(
                    std::io::ErrorKind::Other,
                    err
                )
            })
    }

    async fn remove(&self, key: &str) -> Result<(), Error> {
        let mut conn = self.get_conn().await?;
        conn.del(key).await
            .map_err(|err| {
                Error::new(
                    std::io::ErrorKind::Other,
                    err
                )
            })
    }

    async fn expire(&self, key: &str, expire_time: Duration) -> Result<(), Error> {
        let mut conn = self.get_conn().await?;
        conn.expire(key, expire_time.as_seconds_f32() as i64).await
            .map_err(|err| {
                Error::new(
                    std::io::ErrorKind::Other,
                    err
                )
            })
    }

    async fn clear(&self) -> Result<(), Error> {
        let mut conn = self.get_conn().await?;
        conn.flushall::<()>().await
            .map_err(|err| {
                Error::new(
                    std::io::ErrorKind::Other,
                    err
                )
            })?;
        Ok(())
    }
}