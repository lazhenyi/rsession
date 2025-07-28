use async_trait::async_trait;
use time::Duration;
use crate::SessionInner;

#[async_trait]
pub trait SessionStore: Clone + Sync + Send + 'static  {
    async fn get(&self, key: &str) -> Result<SessionInner, std::io::Error>;
    async fn set(&self, key: &str, value: SessionInner) -> Result<(), std::io::Error>;
    async fn remove(&self, key: &str) -> Result<(), std::io::Error>;
    async  fn expire(&self, key: &str, expire_time: Duration) -> Result<(), std::io::Error>;
    async fn clear(&self) -> Result<(), std::io::Error>;
}


pub enum SessionStoreInner {
    #[cfg(feature = "redis")]
    Redis(deadpool_redis::Pool),
    #[cfg(feature = "redis-cluster")]
    RedisCluster(deadpool_redis::cluster::Pool),
    #[cfg(feature = "redis-sentinel")]
    RedisSentinel(deadpool_redis::sentinel::Pool),
}

#[cfg(feature = "redis")]
pub mod redis;
#[cfg(feature = "redis-cluster")]
pub mod redis_cluster;
#[cfg(feature = "redis-sentinel")]
pub mod redis_sentinel;
