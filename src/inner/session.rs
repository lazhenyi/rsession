use crate::SessionInner;
use std::cell::RefCell;
use std::io;
use std::rc::Rc;

/// Thread-safe wrapper around session data with interior mutability
///
/// This struct provides the public API for session manipulation while ensuring
/// safe concurrent access through `Rc<RefCell<SessionInner>>`.
#[derive(Clone, Debug)]
pub struct Session(pub(crate) Rc<RefCell<SessionInner>>);

/// Unsafe Send implementation for Session
///
/// Safety: While `RefCell` is not thread-safe, this implementation is valid
/// because the session system ensures proper synchronization in async contexts.
unsafe impl Send for Session {}
/// Unsafe Sync implementation for Session
///
/// Safety: See Send implementation for safety justification
unsafe impl Sync for Session {}
impl Session {
    /// Creates a new Session from an `Rc<RefCell<SessionInner>>`
    ///
    /// # Arguments
    /// * `inner` - Shared reference to the underlying session data
    pub fn new(inner: Rc<RefCell<SessionInner>>) -> Self {
        Session(inner)
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
    /// Ok(T) if the key exists and deserialization succeeds, Err(io::Error) otherwise
    pub fn get<T>(&self, key: &str) -> Result<T, io::Error>
    where
        T: serde::de::DeserializeOwned,
    {
        self.0
            .borrow()
            .get::<T>(key)
            .ok_or(io::Error::new(io::ErrorKind::Other, "get session error"))
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
    pub fn set<T>(&self, key: &str, value: T) -> Result<(), io::Error>
    where
        T: serde::Serialize,
    {
        self.0.borrow_mut().set(key, value)
    }
    /// Removes a key-value pair from the session
    ///
    /// # Arguments
    /// * `key` - The key to remove from the session data
    pub fn remove(&self, key: &str) {
        self.0.borrow_mut().remove(key)
    }
    /// Clears all data from the session
    ///
    /// This sets the session status to Clear, triggering full removal from storage
    pub fn clear(&self) {
        self.0.borrow_mut().clear()
    }
    /// Returns the number of key-value pairs in the session
    ///
    /// # Returns
    /// The count of entries in the session data map
    pub fn len(&self) -> usize {
        self.0.borrow().len()
    }
    /// Returns a cloned copy of the inner SessionInner data
    ///
    /// # Returns
    /// A clone of the underlying SessionInner containing all session data
    pub fn inner(&self) -> SessionInner {
        self.0.borrow().clone()
    }
}
