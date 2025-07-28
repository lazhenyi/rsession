//! Session storage backend interface and implementations
//!
//! This module defines the common interface for session storage backends and
//! provides Redis-based implementations through feature flags.
use crate::SessionInner;
use async_trait::async_trait;
use time::Duration;

/// Common interface for session storage backends
///
/// This trait defines the required operations for persistent session storage
/// and must be implemented by all storage backends. Implementations must be
/// thread-safe (Sync + Send) and cloneable for use across async tasks.
#[async_trait]
pub trait SessionStore: Clone + Sync + Send + 'static {
    /// Retrieves a session from storage by key
    ///
    /// # Arguments
    /// * `key` - Session identifier to look up
    ///
    /// # Returns
    /// Ok(SessionInner) if found, Err(io::Error) if retrieval fails
    async fn get(&self, key: &str) -> Result<SessionInner, std::io::Error>;
    /// Stores a session in storage with the given key
    ///
    /// # Arguments
    /// * `key` - Session identifier to associate with the session
    /// * `value` - SessionInner instance containing the session data
    ///
    /// # Returns
    /// Ok(()) if successful, Err(io::Error) if storage fails
    async fn set(&self, key: &str, value: SessionInner) -> Result<(), std::io::Error>;
    /// Removes a specific session from storage
    ///
    /// # Arguments
    /// * `key` - Session identifier to remove
    ///
    /// # Returns
    /// Ok(()) if successful, Err(io::Error) if removal fails
    async fn remove(&self, key: &str) -> Result<(), std::io::Error>;
    /// Sets expiration duration for a session
    ///
    /// # Arguments
    /// * `key` - Session identifier to update
    /// * `expire_time` - Duration until the session expires
    ///
    /// # Returns
    /// Ok(()) if successful, Err(io::Error) if expiration update fails
    async fn expire(&self, key: &str, expire_time: Duration) -> Result<(), std::io::Error>;
    /// Removes all sessions from storage
    ///
    /// # Warning
    /// This is a destructive operation that will delete all session data
    ///
    /// # Returns
    /// Ok(()) if successful, Err(io::Error) if clear operation fails
    async fn clear(&self) -> Result<(), std::io::Error>;
}

/// Type-erased wrapper for different Redis connection pools
///
/// This enum encapsulates various Redis connection pool implementations
/// based on enabled features, providing a unified storage interface.
pub enum SessionStoreInner {
    #[cfg(feature = "redis")]
    /// Standard Redis connection pool
    ///
    /// Enabled with the "redis" feature flag
    Redis(deadpool_redis::Pool),
    #[cfg(feature = "redis-cluster")]
    /// Redis Cluster connection pool
    ///
    /// Enabled with the "redis-cluster" feature flag
    RedisCluster(deadpool_redis::cluster::Pool),
    #[cfg(feature = "redis-sentinel")]
    /// Redis Sentinel connection pool
    ///
    /// Enabled with the "redis-sentinel" feature flag
    RedisSentinel(deadpool_redis::sentinel::Pool),
}

#[cfg(feature = "redis")]
pub mod redis;
#[cfg(feature = "redis-cluster")]
pub mod redis_cluster;
#[cfg(feature = "redis-sentinel")]
pub mod redis_sentinel;
