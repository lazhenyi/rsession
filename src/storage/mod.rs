use std::collections::HashMap;
use time::Duration;

pub trait SessionStore: Clone + Sync + Send + 'static  {
    fn get(&self, key: &str) -> impl Future<Output = Result<HashMap<String,String>, std::io::Error>>;
    fn set(&self, key: &str, value: HashMap<String,String>) -> impl Future<Output = Result<(), std::io::Error>>;
    fn remove(&self, key: &str) -> impl Future<Output = Result<(), std::io::Error>>;
    fn expire(&self, key: &str, expire_time: Duration) -> impl Future<Output = Result<(), std::io::Error>>;
    fn clear(&self) -> impl Future<Output = Result<(), std::io::Error>>;
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
