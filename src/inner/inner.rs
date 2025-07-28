use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io;
use uuid::Uuid;
use crate::SessionStatus::Change;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum SessionStatus {
    UnChange,
    Change,
    Clear,
    Destroy,
    Expire,
}
impl Default for SessionStatus {
    fn default() -> Self {
        SessionStatus::UnChange
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SessionInner {
    #[serde(skip)]
    pub(crate) id: String,
    pub(crate) data: HashMap<String, String>,
    #[serde(skip)]
    pub(crate) status: SessionStatus,
}

impl Default for SessionInner {
    fn default() -> Self {
        SessionInner {
            id: Uuid::now_v7().to_string(),
            data: HashMap::new(),
            status: SessionStatus::UnChange,
        }
    }
}

impl SessionInner {
    pub fn new(id: String) -> Self {
        let mut this = SessionInner::default();
        this.id = id;
        this.status = Change;
        this
    }
    pub fn get<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.data.get(key)
            .and_then(|s| serde_json::from_str::<T>(s).ok())
    }
    pub fn set<T: Serialize>(&mut self, key: &str, value: T) -> Result<(), io::Error>{
        if let Ok(s) = serde_json::to_string(&value) {
            self.data.insert(key.to_string(), s);
            self.status = Change;
            Ok(())
        } else {
            Err(io::Error::new(io::ErrorKind::Other, "serde_json::to_string failed"))
        }
    }
    pub fn remove(&mut self, key: &str) {
        self.data.remove(key);
        self.status = Change;
    }
    pub fn clear(&mut self) {
        self.data.clear();
        self.status = SessionStatus::Clear;
    }
    pub fn len(&self) -> usize {
        self.data.len()
    }
}