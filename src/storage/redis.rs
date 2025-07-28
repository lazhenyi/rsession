use crate::storage::SessionStore;
use deadpool_redis::redis::AsyncCommands;
use std::collections::HashMap;
use std::io::Error;
use async_trait::async_trait;
use time::Duration;
use crate::{RandKey, SessionInner};

#[derive(Clone)]
pub struct RedisSessionStorage {
    pub conn: deadpool_redis::Pool,
    pub rand_key: RandKey,
    pub prefix: String,

}



impl RedisSessionStorage {
    pub fn new(pool: deadpool_redis::Pool,rand_key: RandKey) -> Self {
        RedisSessionStorage {
            conn: pool,
            rand_key,
            prefix: "".to_string(),
        }
    }
    pub fn set_prefix(&mut self, prefix: &str) -> Self {
        self.prefix = prefix.to_string();
        RedisSessionStorage {
            conn: self.conn.clone(),
            rand_key: self.rand_key.clone(),
            prefix: self.prefix.clone(),
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

#[async_trait]
impl SessionStore for RedisSessionStorage {
    async fn get(&self, key: &str) -> Result<SessionInner, Error> {
        let mut conn = self.get_conn().await?;
        let value = conn.get::<&str, String>(&format!("{}{}", self.prefix, key)).await
            .map_err(|err| {
                Error::new(
                    std::io::ErrorKind::Other,
                    err
                )
            });
        let map = value.map(|x| {
            serde_json::from_str::<HashMap<String, String>>(&x)
                .map_err(|err| {
                    Error::new(
                        std::io::ErrorKind::Other,
                        err
                    )
                })
        })?
            .unwrap_or(HashMap::new());
        let mut inner = SessionInner::new(self.rand_key.generate());
        inner.data = map;
        Ok(inner)
            
    }

    async fn set(&self, key: &str, value: SessionInner) -> Result<(), Error> {
        let mut conn = self.get_conn().await?;
        conn.set(&format!("{}{}", self.prefix, key), serde_json::to_string(&value.data)?).await
            .map_err(|err| {
                Error::new(
                    std::io::ErrorKind::Other,
                    err
                )
            })
    }

    async fn remove(&self, key: &str) -> Result<(), Error> {
        let mut conn = self.get_conn().await?;
        conn.del(&format!("{}{}", self.prefix, key)).await
            .map_err(|err| {
                Error::new(
                    std::io::ErrorKind::Other,
                    err
                )
            })
    }

    async fn expire(&self, key: &str, expire_time: Duration) -> Result<(), Error> {
        let mut conn = self.get_conn().await?;
        conn.expire(&format!("{}{}", self.prefix, key), expire_time.as_seconds_f32() as i64).await
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