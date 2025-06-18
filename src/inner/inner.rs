use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use cookie::Cookie;
use serde::de::DeserializeOwned;
use crate::SessionBuilder;

#[derive(Clone,Debug)]
pub enum SessionStatus {
    UnChange,
    Change,
    Clear,
    Destroy,
    Expire,
}


pub struct Session(Rc<RefCell<SessionInner>>);

impl Session {
    pub fn new(inner: Rc<RefCell<SessionInner>>) -> Self {
        Self(inner)
    }
}


#[derive(Clone,Debug)]
pub struct SessionInner {
    pub(crate) session_id: String,
    pub(crate) status: SessionStatus,
    pub(crate) data: HashMap<String,String>,
}


impl SessionInner {
    pub(crate) fn new(session_id: String) -> Self {
        SessionInner {
            session_id,
            status: SessionStatus::UnChange,
            data: HashMap::new(),
        }
    }
    pub fn session_id(&self) -> String {
        self.session_id.clone()
    }
   
    pub(crate) fn builder(&self, config: &Rc<SessionBuilder>) -> Cookie {
        let mut cookie = Cookie::new(config.key.clone(), self.session_id.clone());
        cookie.set_path(config.path.clone());
        cookie.set_domain(config.domain.clone());
        cookie.set_secure(config.secure);
        cookie.set_http_only(config.http_only);
        cookie.set_max_age(config.max_age);
        if let Some(x) = config.same_site {
            cookie.set_same_site(x);
        }
        cookie.set_expires(time::OffsetDateTime::now_utc() + config.expire_time);
        cookie
    }
}


impl Session {
    pub fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>, std::io::Error> {
        Ok(
            match self.0.borrow_mut().data.get(key) {
                Some(x) => {
                    Some(
                        serde_json::from_slice(x.as_bytes()).map_err(|x|{
                            std::io::Error::new(
                                std::io::ErrorKind::Other,
                                x
                            )
                        })?
                    )
                },
                None => None
            }
        )
    }
    pub fn set<T: serde::Serialize>(&self, key: &str, value: T) -> Result<(), std::io::Error> {
        self.0.borrow_mut().status = SessionStatus::Change;
        self.0.borrow_mut().data.insert(
            key.to_string(),
            serde_json::to_string(&value).map_err(|x|{
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    x
                )
            })?
        );
        Ok(())
    }
    pub fn remove(&self, key: &str) -> Result<(), std::io::Error> {
        self.0.borrow_mut().status = SessionStatus::Change;
        self.0.borrow_mut().data.remove(key);
        Ok(())
    }
    pub fn clear(&self) -> Result<(), std::io::Error> {
        self.0.borrow_mut().status = SessionStatus::Clear;
        self.0.borrow_mut().data.clear();
        Ok(())
    }
    pub fn destroy(&self) -> Result<(), std::io::Error> {
        self.0.borrow_mut().status = SessionStatus::Destroy;
        Ok(())
    }
}

