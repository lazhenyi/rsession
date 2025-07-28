use cookie::{Cookie, Expiration, SameSite};
use rand::Rng;
use sha256::Sha256Digest;
use std::ops::Add;
use std::rc::Rc;
use time::{Duration, OffsetDateTime};

#[derive(Debug,Clone)]
pub enum RefreshStrategy {
    BrowserLifeCycle,
    PersistentStorage(Duration),
}

#[derive(Debug,Clone)]
pub enum RandKey {
    Random(usize),
    UuidV4,
    UuidV7,
    RandomSha256(usize),
}
impl Default for RandKey {
    fn default() -> Self {
        RandKey::RandomSha256(32)
    }
}

impl RandKey {
    pub fn generate(&self) -> String { 
        let this = self.clone();
        match this {
            RandKey::Random(len) => {
                let mut rng = rand::rng();
                let bytes: f64 = rng.random::<f64>() % ( 10 * len ) as f64;
                bytes.to_string()
            },
            RandKey::UuidV4 => {
                uuid::Uuid::new_v4().to_string()
            },
            RandKey::UuidV7 => {
                uuid::Uuid::now_v7().to_string()
            }
            RandKey::RandomSha256(len) => {
                let mut rng = rand::rng();
                let bytes: f64 = rng.random::<f64>() % ( 10 * len ) as f64;
                bytes.to_string().digest()
            }
        }
    }
}

#[derive(Debug,Clone)]
pub struct SessionBuilder {
    pub key: String,
    pub secret: Option<[u8;64]>,
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
    pub fn new() -> Self {
        SessionBuilder::default()
    }
    pub fn key(mut self, key: &str) -> Self {
        self.key = key.to_string();
        self
    }
    pub fn secret(mut self, secret: &[u8]) -> Self {
        assert_eq!(secret.len(), 64, "secret must be 64 bytes");
        self.secret = Some(<[u8; 64]>::try_from(secret.to_vec()).unwrap());
        self
    }
    pub fn expire_time(mut self, expire_time: Duration) -> Self {
        self.expire_time = expire_time;
        self
    }
    pub fn path(mut self, path: &str) -> Self {
        self.path = path.to_string();
        self
    }
    pub fn domain(mut self, domain: &str) -> Self {
        self.domain = domain.to_string();
        self
    }
    pub fn secure(mut self, secure: bool) -> Self {
        self.secure = secure;
        self
    }
    pub fn http_only(mut self, http_only: bool) -> Self {
        self.http_only = http_only;
        self
    }
    pub fn max_age(mut self, max_age: Duration) -> Self {
        self.max_age = Some(max_age);
        self
    }
    pub fn same_site(mut self, same_site: SameSite) -> Self {
        self.same_site = Some(same_site);
        self
    }
    pub fn refresh_strategy(mut self, refresh_strategy: RefreshStrategy) -> Self {
        self.refresh_strategy = refresh_strategy;
        self
    }
    pub fn rand_key(mut self, rand_key: RandKey) -> Self {
        match rand_key {
            RandKey::Random(len) => {
                assert!(len > 64, "len must be greater than 64");
                assert!(len < 1024, "len must be less than 1024");
            },
            RandKey::UuidV4 | RandKey::UuidV7 => {},
            RandKey::RandomSha256(len) => {
                assert!(len > 64, "len must be greater than 64");
                assert!(len < 1024, "len must be less than 1024");
            }
        }
        self.rand_key = Rc::from(rand_key);
        self
    }

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
                cookie.set_expires(Expiration::DateTime(OffsetDateTime::now_utc().add(duration)));
            }
        }
        if let Some(max_age) = self.max_age {
            cookie.set_max_age(max_age);
        }
        cookie
    }
}
