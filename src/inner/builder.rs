//! Session configuration and cookie building utilities
//!
//! This module provides types and utilities for configuring session behavior
//! and building cookies according to the specified configuration.

use cookie::{Cookie, Expiration, SameSite};
use rand::Rng;
use sha256::Sha256Digest;
use std::ops::Add;
use std::rc::Rc;
use time::{Duration, OffsetDateTime};

/// Strategy for determining how long a session should persist
///
/// Controls whether sessions expire when the browser closes or after a fixed duration
#[derive(Debug, Clone)]
pub enum RefreshStrategy {
    /// Session persists only for the browser session
    ///
    /// Cookie will be deleted when the browser closes
    BrowserLifeCycle,
    /// Session persists for a fixed duration regardless of browser activity
    ///
    /// # Tuple Fields
    /// * `0` - Duration for which the session will remain valid
    PersistentStorage(Duration),
}

/// Strategy for generating session IDs
///
/// Defines different methods for creating unique session identifiers
#[derive(Debug, Clone)]
pub enum RandKey {
    /// Generate random numeric string with specified length
    ///
    /// # Tuple Fields
    /// * `0` - Length of the random numeric string to generate
    Random(usize),
    /// Generate UUID v4 compliant identifier
    ///
    /// Uses random numbers to create a universally unique identifier
    UuidV4,
    /// Generate UUID v7 compliant identifier
    ///
    /// Uses timestamp and random numbers for better indexing performance
    UuidV7,
    /// Generate random string and hash it with SHA-256
    ///
    /// # Tuple Fields
    /// * `0` - Length of the input random string before hashing
    RandomSha256(usize),
}
impl Default for RandKey {
    fn default() -> Self {
        RandKey::RandomSha256(32)
    }
}

impl RandKey {
    /// Generates a new session ID string based on the selected strategy
    pub fn generate(&self) -> String {
        let this = self.clone();
        match this {
            RandKey::Random(len) => {
                let mut rng = rand::rng();
                let bytes: f64 = rng.random::<f64>() % (10 * len) as f64;
                bytes.to_string()
            }
            RandKey::UuidV4 => uuid::Uuid::new_v4().to_string(),
            RandKey::UuidV7 => uuid::Uuid::now_v7().to_string(),
            RandKey::RandomSha256(len) => {
                let mut rng = rand::rng();
                let bytes: f64 = rng.random::<f64>() % (10 * len) as f64;
                bytes.to_string().digest()
            }
        }
    }
}

/// Builder for configuring session behavior and creating cookies
///
/// Provides a fluent interface for setting session parameters and generating
/// properly configured cookies for client-side storage.
#[derive(Debug, Clone)]
pub struct SessionBuilder {
    pub key: String,
    pub secret: Option<[u8; 64]>,
    pub expire_time: Duration,
    pub path: String,
    pub domain: String,
    pub secure: bool,
    pub http_only: bool,
    pub max_age: Option<Duration>,
    pub same_site: Option<SameSite>,
    pub refresh_strategy: RefreshStrategy,
    pub rand_key: Rc<RandKey>,
    pub auto_expire: bool,
}

unsafe impl Sync for SessionBuilder {}
unsafe impl Send for SessionBuilder {}

impl Default for SessionBuilder {
    fn default() -> Self {
        SessionBuilder {
            key: "session_key".to_string(),
            secret: None,
            expire_time: Duration::days(7),
            path: "/".to_string(),
            domain: "".to_string(),
            secure: true,
            http_only: true,
            max_age: None,
            same_site: None,
            refresh_strategy: RefreshStrategy::BrowserLifeCycle,
            rand_key: Rc::new(RandKey::UuidV7),
            auto_expire: true,
        }
    }
}

impl SessionBuilder {
    /// Creates a new SessionBuilder with default configuration
    ///
    /// Default settings:
    /// - Cookie name: "session_key"
    /// - Expiration: 7 days
    /// - Path: "/"
    /// - Secure: true
    /// - HTTP-only: true
    /// - ID generation: UuidV7
    pub fn new() -> Self {
        SessionBuilder::default()
    }
    /// Sets the cookie name for the session ID
    ///
    /// # Arguments
    /// * `key` - Name to use for the session cookie
    pub fn key(mut self, key: &str) -> Self {
        self.key = key.to_string();
        self
    }
    /// Sets the secret key for session encryption (64 bytes required)
    ///
    /// # Arguments
    /// * `secret` - 64-byte array used for cryptographic operations
    ///
    /// # Panics
    /// Panics if the secret is not exactly 64 bytes long
    pub fn secret(mut self, secret: &[u8]) -> Self {
        assert_eq!(secret.len(), 64, "secret must be 64 bytes");
        self.secret = Some(<[u8; 64]>::try_from(secret.to_vec()).unwrap());
        self
    }
    /// Sets the duration after which the session expires
    ///
    /// # Arguments
    /// * `expire_time` - Time until session becomes invalid
    pub fn expire_time(mut self, expire_time: Duration) -> Self {
        self.expire_time = expire_time;
        self
    }
    /// Sets the URL path for which the cookie is valid
    ///
    /// # Arguments
    /// * `path` - Path pattern that must match for the cookie to be sent
    pub fn path(mut self, path: &str) -> Self {
        self.path = path.to_string();
        self
    }
    /// Sets the domain for which the cookie is valid
    ///
    /// # Arguments
    /// * `domain` - Domain name that must match for the cookie to be sent
    pub fn domain(mut self, domain: &str) -> Self {
        self.domain = domain.to_string();
        self
    }
    /// Sets whether the cookie requires a secure (HTTPS) connection
    ///
    /// # Arguments
    /// * `secure` - If true, cookie will only be sent over HTTPS
    pub fn secure(mut self, secure: bool) -> Self {
        self.secure = secure;
        self
    }
    /// Sets whether the cookie is accessible only through HTTP(S)
    ///
    /// If true, the cookie cannot be accessed by client-side JavaScript
    ///
    /// # Arguments
    /// * `http_only` - Enable/disable HTTP-only flag
    pub fn http_only(mut self, http_only: bool) -> Self {
        self.http_only = http_only;
        self
    }
    /// Sets the maximum age of the cookie
    ///
    /// # Arguments
    /// * `max_age` - Duration indicating how long the cookie should persist
    pub fn max_age(mut self, max_age: Duration) -> Self {
        self.max_age = Some(max_age);
        self
    }
    /// Sets the SameSite policy for the cookie
    ///
    /// Controls when cookies are sent with cross-site requests
    ///
    /// # Arguments
    /// * `same_site` - SameSite policy enum from the cookie crate
    pub fn same_site(mut self, same_site: SameSite) -> Self {
        self.same_site = Some(same_site);
        self
    }
    /// Sets the session persistence strategy
    ///
    /// # Arguments
    /// * `refresh_strategy` - Strategy for determining session lifetime
    pub fn refresh_strategy(mut self, refresh_strategy: RefreshStrategy) -> Self {
        self.refresh_strategy = refresh_strategy;
        self
    }
    /// Sets the session ID generation strategy
    ///
    /// # Arguments
    /// * `rand_key` - Strategy for generating unique session identifiers
    pub fn rand_key(mut self, rand_key: RandKey) -> Self {
        match rand_key {
            RandKey::Random(len) => {
                assert!(len > 64, "len must be greater than 64");
                assert!(len < 1024, "len must be less than 1024");
            }
            RandKey::UuidV4 | RandKey::UuidV7 => {}
            RandKey::RandomSha256(len) => {
                assert!(len > 64, "len must be greater than 64");
                assert!(len < 1024, "len must be less than 1024");
            }
        }
        self.rand_key = Rc::from(rand_key);
        self
    }

    /// Builds a cookie with the configured parameters and given session ID
    ///
    /// # Arguments
    /// * `id` - Session ID to be stored in the cookie
    ///
    /// # Returns
    /// A configured Cookie instance ready to be sent to the client
    pub fn build(&self, id: String) -> Cookie {
        let mut cookie = Cookie::new(self.key.clone(), id);
        cookie.set_domain(self.domain.clone());
        cookie.set_path(self.path.clone());
        cookie.set_http_only(self.http_only);
        cookie.set_secure(self.secure);
        cookie.set_same_site(self.same_site);
        match self.refresh_strategy {
            RefreshStrategy::BrowserLifeCycle => {
                cookie.unset_expires();
            }
            RefreshStrategy::PersistentStorage(duration) => {
                cookie.set_expires(Expiration::DateTime(
                    OffsetDateTime::now_utc().add(duration),
                ));
            }
        }
        if let Some(max_age) = self.max_age {
            cookie.set_max_age(max_age);
        }
        cookie
    }
}
