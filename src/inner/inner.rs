use crate::SessionStatus::Change;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io;
use uuid::Uuid;

/// Tracks the modification state of a session
///
/// This enum is used internally to determine how to persist session changes
/// to storage and when to update client cookies.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum SessionStatus {
    /// Session data has not been modified
    ///
    /// No persistence operations will be performed for this session
    UnChange,
    ///
    /// Session will be persisted to storage and cookie will be updated
    Change,
    /// Session data has been cleared
    ///
    /// All session data will be removed from storage
    Clear,
    /// Session has been marked for destruction
    ///
    /// Session will be completely removed from storage
    Destroy,
    /// Session expiration has been updated
    ///
    /// Storage expiration timestamp will be refreshed
    Expire,
}
impl Default for SessionStatus {
    fn default() -> Self {
        SessionStatus::UnChange
    }
}

/// Internal representation of session data
///
/// Contains the session identifier, stored data, and modification status
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SessionInner {
    /// Unique session identifier
    #[serde(skip)]
    pub(crate) id: String,
    /// Serialized session data stored as key-value pairs
    pub(crate) data: HashMap<String, String>,
    /// Current modification status of the session
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
    /// Creates a new SessionInner with the specified ID
    ///
    /// # Arguments
    /// * `id` - Unique identifier for the session
    pub fn new(id: String) -> Self {
        let mut this = SessionInner::default();
        this.id = id;
        this.status = Change;
        this
    }
    /// Retrieves and deserializes a value from the session
    ///
    /// # Arguments
    /// * `key` - The key associated with the value to retrieve
    ///
    /// # Type Parameters
    /// * `T` - The type to deserialize the value into
    ///
    /// # Returns
    /// Some(T) if the key exists and deserialization succeeds, None otherwise
    pub fn get<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.data
            .get(key)
            .and_then(|s| serde_json::from_str::<T>(s).ok())
    }
    /// Serializes and stores a value in the session
    ///
    /// # Arguments
    /// * `key` - The key to associate with the value
    /// * `value` - The value to serialize and store
    ///
    /// # Type Parameters
    /// * `T` - The type of the value to store (must implement Serialize)
    ///
    /// # Returns
    /// Ok(()) if successful, Err(io::Error) if serialization fails
    pub fn set<T: Serialize>(&mut self, key: &str, value: T) -> Result<(), io::Error> {
        if let Ok(s) = serde_json::to_string(&value) {
            self.data.insert(key.to_string(), s);
            self.status = Change;
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::Other,
                "serde_json::to_string failed",
            ))
        }
    }
    /// Removes a key-value pair from the session
    ///
    /// # Arguments
    /// * `key` - The key to remove from the session data
    pub fn remove(&mut self, key: &str) {
        self.data.remove(key);
        self.status = Change;
    }
    /// Clears all data from the session
    ///
    /// Sets status to Clear, which will trigger full removal from storage
    pub fn clear(&mut self) {
        self.data.clear();
        self.status = SessionStatus::Clear;
    }
    /// Returns the number of key-value pairs in the session
    ///
    /// # Returns
    /// The count of entries in the session data map
    pub fn len(&self) -> usize {
        self.data.len()
    }
}
