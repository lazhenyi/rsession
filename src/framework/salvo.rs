//! Salvo session management integration
//!
//! This module provides Salvo framework integration for session management,
//! including middleware and extension traits for easy session access.
use crate::{Session, SessionBuilder, SessionInner, SessionStatus, SessionStore};
use async_trait::async_trait;
use http::header::SET_COOKIE;
use salvo::{Depot, FlowCtrl, Handler, Request, Response};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

/// Salvo middleware for session management
///
/// This handler integrates session functionality into Salvo applications,
/// handling session creation, storage operations, and cookie management.
///
/// # Type Parameters
/// * `Storage` - The session storage backend implementing SessionStore
pub struct SalvoSessionMiddleware<Storage>
where
    Storage: SessionStore + 'static + Send + Sync + Clone,
{
    builder: Arc<SessionBuilder>,
    store: Arc<Storage>,
}
impl<Storage> SalvoSessionMiddleware<Storage>
where
    Storage: SessionStore + 'static + Send + Sync + Clone,
{
    /// Creates a new SalvoSessionMiddleware
    ///
    /// # Arguments
    /// * `builder` - Session configuration builder with cookie/session settings
    /// * `store` - Session storage backend implementation
    pub fn new(builder: SessionBuilder, store: Storage) -> Self {
        Self {
            builder: Arc::new(builder),
            store: Arc::new(store),
        }
    }
}

#[async_trait]
/// Salvo Handler implementation for session middleware
///
/// Processes incoming requests to load existing sessions or create new ones,
/// injects the session into the Depot, and handles response processing to
/// persist session changes and update client cookies.
#[async_trait]
impl<Storage> Handler for SalvoSessionMiddleware<Storage>
where
    Storage: SessionStore + 'static + Send + Sync + Clone,
{
    async fn handle(
        &self,
        req: &mut Request,
        depot: &mut Depot,
        res: &mut Response,
        ctrl: &mut FlowCtrl,
    ) {
        let builder = self.builder.clone();
        let store = self.store.clone();
        let cookies = req.cookies();
        let inner = match cookies.get(&self.builder.key) {
            Some(cookie) => {
                let session_id = cookie.value();
                store
                    .get(session_id)
                    .await
                    .unwrap_or(SessionInner::new(session_id.to_string()))
            }
            None => SessionInner::default(),
        };
        let session = Session::new(Rc::new(RefCell::new(inner)));
        depot.inject(session.clone());
        ctrl.call_next(req, depot, res).await;
        let inner = session.inner();
        match inner.status {
            SessionStatus::UnChange => {
                if builder.auto_expire {
                    store
                        .expire(&inner.id.to_string(), builder.expire_time)
                        .await
                        .ok();
                }
                store.set(&inner.id.to_string(), inner.clone()).await.ok();
            }
            SessionStatus::Change => {
                store.remove(&inner.id.to_string()).await.ok();
                store.set(&inner.id.to_string(), inner.clone()).await.ok();
                store
                    .expire(&inner.id.to_string(), builder.expire_time)
                    .await
                    .ok();
            }
            SessionStatus::Clear => {
                store.remove(&inner.id.to_string()).await.ok();
            }
            SessionStatus::Destroy => {
                store.remove(&inner.id.to_string()).await.ok();
            }
            SessionStatus::Expire => {
                store
                    .expire(&inner.id.to_string(), builder.expire_time)
                    .await
                    .ok();
            }
        }
        let cookie = builder.build(inner.id.clone());
        if let Ok(cookie) = cookie.to_string().parse() {
            res.headers_mut().insert(SET_COOKIE, cookie);
        }
    }
}

/// Extension trait for Salvo Depot to access session data
///
/// Provides convenient methods to retrieve the session from the request Depot
pub trait SessionDepotExt {
    /// Retrieves the session from the Depot
    ///
    /// # Returns
    /// Some(Session) if a session exists in the Depot, None otherwise
    fn inner_session(&self) -> Option<Session>;
}

impl SessionDepotExt for Depot {
    fn inner_session(&self) -> Option<Session> {
        self.obtain::<Session>().ok().map(|x| x.clone())
    }
}
