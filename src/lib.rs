//! # rsession
//!
//! A simple and flexible session management library for Rust web applications with support for multiple frameworks and storage backends.
//!
//! [![crates.io](https://img.shields.io/crates/v/rsession.svg)](https://crates.io/crates/rsession)
//! [![Released API docs](https://docs.rs/rsession/badge.svg)](https://docs.rs/rsession)
//!
//! ## Features
//!
//! - **Framework Agnostic**: Works with Actix-web, Axum, and Salvo
//! - **Multiple Storage Backends**: Redis, Redis Cluster, and Redis Sentinel support
//! - **Session ID Generation**: UUID v4, UUID v7, Random, and Random SHA256 options
//! - **Configurable**: Expiration times, cookie settings, and refresh strategies
//! - **Type-safe**: Built with Rust's strong type system and serde integration
//!
//! ## Installation
//!
//! Add this to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! rsession = "0.1.0"
//! ```
//!
//! ### Feature Flags
//!
//! Enable specific features based on your needs:
//!
//! ```toml
//! [dependencies.rsession]
//! version = "0.1.0"
//! features = [
//!     "redis",          # Redis storage backend
//!     "redis-cluster",  # Redis Cluster support
//!     "redis-sentinel", # Redis Sentinel support
//!     "actix-web",      # Actix-web framework integration
//!     "tower",          # Axum framework integration
//!     "salvo"           # Salvo framework integration
//! ]
//! ```
//!
//! ## Quick Start
//!
//! ### Axum Example
//!
//! ```rust
//! use axum::routing::get;
//! use axum::Router;
//! use rsession::framework::axum::AxumSessionMiddlewareLayer;
//! use rsession::redis::RedisSessionStorage;
//! use rsession::{RandKey, Session};
//! use tokio::net::TcpListener;
//!
//! #[tokio::main]
//! async fn main() {
//!     // Initialize Redis connection
//!     let redis = deadpool_redis::Config::from_url("redis://localhost:6379")
//!         .create_pool(Some(deadpool_redis::Runtime::Tokio1))
//!         .unwrap();
//!
//!     // Configure session
//!     let session_builder = rsession::SessionBuilder::default()
//!         .expire_time(time::Duration::hours(24))
//!         .rand_key(RandKey::UuidV7);
//!
//!     let store = RedisSessionStorage::new(redis, RandKey::UuidV7);
//!
//!     // Create Axum router with session middleware
//!     let app = Router::new()
//!         .route("/", get(index))
//!         .route_layer(AxumSessionMiddlewareLayer::new(session_builder, store));
//!
//!     // Start server
//!     let listener = TcpListener::bind("0.0.0.0:8080").await.unwrap();
//!     axum::serve(listener, app).await.unwrap();
//! }
//!
//! async fn index(session: Session) -> String {
//!     // Get or initialize counter
//!     let count = session.get::<i32>("count").unwrap_or(0);
//!     session.set("count", count + 1).unwrap();
//!
//!     format!("Current count: {}", count + 1)
//! }
//! ```
//!
//! ## Framework Integrations
//!
//! ### Actix-web
//!
//! ```rust
//! use actix_web::{App, HttpServer, get};
//! use rsession::framework::actix::ActixSessionMiddleware;
//! use rsession::redis::RedisSessionStorage;
//! use rsession::{RandKey, Session};
//!
//! #[get("/")]
//! async fn index(session: Session) -> String {
//!     // Session usage similar to Axum example
//!     // ...
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     // Redis and session configuration
//!     // ...
//!
//!     HttpServer::new(move || {
//!         App::new()
//!             .wrap(ActixSessionMiddleware::new(session_builder, store))
//!             .service(index)
//!     })
//!     .bind("127.0.0.1:8080")?
//!     .run()
//!     .await
//! }
//! ```
//!
//! ### Salvo
//!
//! ```rust
//! use salvo::prelude::*;
//! use rsession::framework::salvo::{SalvoSessionMiddleware, SessionDepotExt};
//! use rsession::redis::RedisSessionStorage;
//! use rsession::RandKey;
//!
//! #[handler]
//! async fn index(depot: &mut Depot) -> String {
//!     let session = depot.inner_session().unwrap();
//!     // Session usage similar to other examples
//!     // ...
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     // Redis and session configuration
//!     // ...
//!
//!     let router = Router::new()
//!         .hoop(SalvoSessionMiddleware::new(session_builder, store))
//!         .get(index);
//!
//!     Server::new(TcpListener::bind("0.0.0.0:8080").bind().await)
//!         .serve(router)
//!         .await;
//! }
//! ```
//!
//! ## Configuration
//!
//! The `SessionBuilder` allows you to customize session behavior:
//!
//! ```rust
//! let session_builder = rsession::SessionBuilder::new()
//!     .key("session_id") // Cookie name
//!     .expire_time(time::Duration::hours(24)) // Session expiration
//!     .path("/") // Cookie path
//!     .domain("example.com") // Cookie domain
//!     .secure(true) // Secure cookie flag
//!     .http_only(true) // HTTP-only cookie flag
//!     .same_site(cookie::SameSite::Lax) // SameSite policy
//!     .rand_key(RandKey::UuidV7) // Session ID generation method
//!     .refresh_strategy(RefreshStrategy::PersistentStorage(time::Duration::days(7)));
//! ```
//!
//! ## Storage Backends
//!
//! ### Redis
//!
//! ```rust
//! let redis = deadpool_redis::Config::from_url("redis://localhost:6379")
//!     .create_pool(Some(deadpool_redis::Runtime::Tokio1))
//!     .unwrap();
//!
//! let mut store = RedisSessionStorage::new(redis, RandKey::UuidV7);
//! store.set_prefix("rsession:"); // Optional key prefix
//! ```
//!
//! ### Redis Cluster
//!
//! ```rust
//! let config = deadpool_redis::cluster::Config::from_urls(&["redis://node1:6379", "redis://node2:6379"]);
//! let pool = config.create_pool(Some(deadpool_redis::Runtime::Tokio1)).unwrap();
//! let store = RedisClusterSessionStorage::new(pool, RandKey::UuidV7);
//! ```
//!
//! ### Redis Sentinel
//!
//! ```rust
//! let config = deadpool_redis::sentinel::Config::new(
//!     "mymaster",
//!     vec!["redis://sentinel1:26379", "redis://sentinel2:26379"],
//! );
//! let pool = config.create_pool(Some(deadpool_redis::Runtime::Tokio1)).unwrap();
//! let store = RedisSentinelSessionStorage::new(pool, RandKey::UuidV7);
//! ```
//!





pub mod framework;
pub mod inner;
pub mod storage;

pub use inner::*;
pub use storage::*;
