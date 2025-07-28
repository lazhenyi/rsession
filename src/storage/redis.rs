//! Redis session storage implementation for rsession
//!
//! This module provides a Redis-backed session store that implements the SessionStore trait.
//! It supports basic session operations with optional key prefixing.

use crate::storage::SessionStore;
use crate::{RandKey, SessionInner};
use async_trait::async_trait;
use deadpool_redis::redis::AsyncCommands;
use std::collections::HashMap;
use std::io::Error;
use time::Duration;

/// Redis-backed session storage implementation
///
/// Uses a connection pool to manage Redis connections and supports key prefixing
/// for namespacing sessions in shared Redis instances.
#[derive(Clone)]
pub struct RedisSessionStorage {
    pub conn: deadpool_redis::Pool,
    pub rand_key: RandKey,
    pub prefix: String,
}

impl RedisSessionStorage {
    /// Creates a new RedisSessionStorage instance
    ///
    /// # Arguments
    /// * `pool` - A deadpool-redis connection pool
    /// * `rand_key` - Strategy for generating random session IDs
    pub fn new(pool: deadpool_redis::Pool, rand_key: RandKey) -> Self {
        RedisSessionStorage {
            conn: pool,
            rand_key,
            prefix: "".to_string(),
        }
    }
    /// Sets the key prefix for Redis storage
    ///
    /// All session keys will be prefixed with this string to avoid key collisions
    /// in shared Redis environments.
    ///
    /// # Arguments
    /// * `prefix` - String to prepend to all Redis keys
    pub fn set_prefix(&mut self, prefix: &str) -> Self {
        self.prefix = prefix.to_string();
        RedisSessionStorage {
            conn: self.conn.clone(),
            rand_key: self.rand_key.clone(),
            prefix: self.prefix.clone(),
        }
    }

    /// Gets a Redis connection from the pool
    ///
    /// Returns a connection from the deadpool or an error if no connections are available
    async fn get_conn(&self) -> Result<deadpool_redis::Connection, Error> {
        match self.conn.get().await {
            Ok(x) => Ok(x),
            Err(x) => Err(Error::new(std::io::ErrorKind::Other, x)),
        }
    }
}

#[async_trait]
impl SessionStore for RedisSessionStorage {
    /// Retrieves a session from Redis by key
    ///
    /// # Arguments
    /// * `key` - Session ID to retrieve
    ///
    /// # Returns
    /// A Result containing the SessionInner if found, or an error if retrieval fails
    async fn get(&self, key: &str) -> Result<SessionInner, Error> {
        let mut conn = self.get_conn().await?;
        let value = conn
            .get::<&str, String>(&format!("{}{}", self.prefix, key))
            .await
            .map_err(|err| Error::new(std::io::ErrorKind::Other, err));
        let map = value
            .map(|x| {
                serde_json::from_str::<HashMap<String, String>>(&x)
                    .map_err(|err| Error::new(std::io::ErrorKind::Other, err))
            })?
            .unwrap_or(HashMap::new());
        let mut inner = SessionInner::new(self.rand_key.generate());
        inner.data = map;
        Ok(inner)
    }

    /// Stores a session in Redis
    ///
    /// # Arguments
    /// * `key` - Session ID to store
    /// * `value` - SessionInner containing the session data
    ///
    /// # Returns
    /// A Result indicating success or failure
    async fn set(&self, key: &str, value: SessionInner) -> Result<(), Error> {
        let mut conn = self.get_conn().await?;
        conn.set(
            &format!("{}{}", self.prefix, key),
            serde_json::to_string(&value.data)?,
        )
        .await
        .map_err(|err| Error::new(std::io::ErrorKind::Other, err))
    }

    /// Removes a session from Redis
    ///
    /// # Arguments
    /// * `key` - Session ID to remove
    ///
    /// # Returns
    /// A Result indicating success or failure
    async fn remove(&self, key: &str) -> Result<(), Error> {
        let mut conn = self.get_conn().await?;
        conn.del(&format!("{}{}", self.prefix, key))
            .await
            .map_err(|err| Error::new(std::io::ErrorKind::Other, err))
    }

    /// Sets an expiration time for a session in Redis
    ///
    /// # Arguments
    /// * `key` - Session ID to set expiration for
    /// * `expire_time` - Duration until the session expires
    ///
    /// # Returns
    /// A Result indicating success or failure
    async fn expire(&self, key: &str, expire_time: Duration) -> Result<(), Error> {
        let mut conn = self.get_conn().await?;
        conn.expire(
            &format!("{}{}", self.prefix, key),
            expire_time.as_seconds_f32() as i64,
        )
        .await
        .map_err(|err| Error::new(std::io::ErrorKind::Other, err))
    }

    /// Clears all sessions from Redis
    ///
    /// WARNING: This will remove ALL keys in the Redis database (uses FLUSHALL)
    /// Use with caution in production environments.
    async fn clear(&self) -> Result<(), Error> {
        let mut conn = self.get_conn().await?;
        conn.flushall::<()>()
            .await
            .map_err(|err| Error::new(std::io::ErrorKind::Other, err))?;
        Ok(())
    }
}
