//! Redis Cluster session storage implementation
//!
//! This module provides a session storage backend using Redis Cluster for distributed
//! session management across multiple Redis nodes.
use crate::storage::SessionStore;
use crate::{RandKey, SessionInner};
use async_trait::async_trait;
use deadpool_redis::redis::AsyncCommands;
use std::collections::HashMap;
use std::io::Error;
use time::Duration;

/// Redis Cluster session storage backend
///
/// This implementation uses Redis Cluster for distributed session storage with
/// connection pooling and key prefixing support.
#[derive(Clone)]
pub struct RedisClusterSessionStorage {
    /// Redis Cluster connection pool
    pub conn: deadpool_redis::cluster::Pool,
    /// Session ID generation strategy
    pub rand_key: RandKey,
    /// Key prefix for namespacing session keys in Redis
    pub prefix: String,
}

impl RedisClusterSessionStorage {
    /// Acquires a connection from the Redis Cluster pool
    ///
    /// # Returns
    /// Ok(Connection) if successful, Err(io::Error) if connection fails
    async fn get_conn(&self) -> Result<deadpool_redis::cluster::Connection, Error> {
        match self.conn.get().await {
            Ok(x) => Ok(x),
            Err(x) => Err(Error::new(std::io::ErrorKind::Other, x)),
        }
    }
}

#[async_trait]
impl SessionStore for RedisClusterSessionStorage {
    /// Retrieves a session from Redis Cluster
    ///
    /// # Arguments
    /// * `key` - Session identifier to look up
    ///
    /// # Returns
    /// Ok(SessionInner) if found and deserialized successfully, Err(io::Error) otherwise
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

    /// Stores a session in Redis Cluster
    ///
    /// # Arguments
    /// * `key` - Session identifier to associate with the session data
    /// * `value` - SessionInner instance containing the data to store
    ///
    /// # Returns
    /// Ok(()) if successful, Err(io::Error) if serialization or storage fails
    async fn set(&self, key: &str, value: SessionInner) -> Result<(), Error> {
        let mut conn = self.get_conn().await?;
        conn.set(
            &format!("{}{}", self.prefix, key),
            serde_json::to_string(&value.data)?,
        )
        .await
        .map_err(|err| Error::new(std::io::ErrorKind::Other, err))
    }
    /// Removes a session from Redis Cluster
    ///
    /// # Arguments
    /// * `key` - Session identifier to remove
    ///
    /// # Returns
    /// Ok(()) if successful, Err(io::Error) if removal fails
    async fn remove(&self, key: &str) -> Result<(), Error> {
        let mut conn = self.get_conn().await?;
        conn.del(&format!("{}{}", self.prefix, key))
            .await
            .map_err(|err| Error::new(std::io::ErrorKind::Other, err))
    }

    /// Sets expiration time for a session in Redis Cluster
    ///
    /// # Arguments
    /// * `key` - Session identifier to update
    /// * `expire_time` - Duration until the session expires
    ///
    /// # Returns
    /// Ok(()) if successful, Err(io::Error) if expiration update fails
    async fn expire(&self, key: &str, expire_time: Duration) -> Result<(), Error> {
        let mut conn = self.get_conn().await?;
        conn.expire(
            &format!("{}{}", self.prefix, key),
            expire_time.as_seconds_f32() as i64,
        )
        .await
        .map_err(|err| Error::new(std::io::ErrorKind::Other, err))
    }

    /// Removes all sessions from Redis Cluster
    ///
    /// # Warning
    /// This is a destructive operation that will clear ALL data in the Redis Cluster
    /// using FLUSHALL
    ///
    /// # Returns
    /// Ok(()) if successful, Err(io::Error) if clear operation fails
    async fn clear(&self) -> Result<(), Error> {
        let mut conn = self.get_conn().await?;
        conn.flushall::<()>()
            .await
            .map_err(|err| Error::new(std::io::ErrorKind::Other, err))?;
        Ok(())
    }
}
