use std::cell::RefCell;
use std::io;
use std::rc::Rc;
use crate::SessionInner;

#[derive(Clone,Debug)]
pub struct Session(pub(crate) Rc<RefCell<SessionInner>>);

unsafe impl Send for Session {}
unsafe impl Sync for Session {}
impl Session {
    pub fn new(inner: Rc<RefCell<SessionInner>>) -> Self {
        Session(inner)
    }
    pub fn get<T>(&self, key: &str) -> Result<T, io::Error> where T: serde::de::DeserializeOwned {
        self.0.borrow().get::<T>(key)
            .ok_or(io::Error::new(io::ErrorKind::Other, "get session error"))
    }
    pub fn set<T>(&self, key: &str, value: T) -> Result<(), io::Error> where T: serde::Serialize {
        self.0.borrow_mut().set(key, value)
    }
    pub fn remove(&self, key: &str) {
        self.0.borrow_mut().remove(key)
    }
    pub fn clear(&self) {
        self.0.borrow_mut().clear()
    }
    pub fn len(&self) -> usize {
        self.0.borrow().len()
    }
    pub(crate) fn inner(&self) -> SessionInner {
        self.0.borrow().clone()
    }

}