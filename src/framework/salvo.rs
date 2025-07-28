use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use async_trait::async_trait;
use http::header::SET_COOKIE;
use salvo::{Depot, FlowCtrl, Handler, Request, Response};
use crate::{Session, SessionBuilder, SessionInner, SessionStatus, SessionStore};

pub struct SalvoSessionMiddleware<Storage>
where
    Storage: SessionStore + 'static + Send + Sync + Clone,
{
    builder: Arc<SessionBuilder>,
    store: Arc<Storage>,
}
impl <Storage> SalvoSessionMiddleware<Storage>
where
    Storage: SessionStore + 'static + Send + Sync + Clone,
{
    pub fn new(builder: SessionBuilder, store: Storage) -> Self {
        Self {
            builder: Arc::new(builder),
            store: Arc::new(store),
        }
    }
}

#[async_trait]
impl<Storage> Handler for SalvoSessionMiddleware<Storage>
where Storage: SessionStore + 'static + Send + Sync + Clone,
{
    async fn handle(&self, req: &mut Request, depot: &mut Depot, res: &mut Response, ctrl: &mut FlowCtrl) {
        let builder = self.builder.clone();
        let store = self.store.clone();
        let cookies = req.cookies();
        let inner = match cookies.get(&self.builder.key) {
            Some(cookie) => {
                let session_id = cookie.value();
                store.get(session_id).await.unwrap_or(SessionInner::new(session_id.to_string()))
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
                    store.expire(&inner.id.to_string(), builder.expire_time).await.ok();
                }
                store.set(&inner.id.to_string(), inner.clone()).await.ok();
            }
            SessionStatus::Change => {
                store.remove(&inner.id.to_string()).await.ok();
                store.set(&inner.id.to_string(), inner.clone()).await.ok();
                store.expire(&inner.id.to_string(), builder.expire_time).await.ok();
            }
            SessionStatus::Clear => {
                store.remove(&inner.id.to_string()).await.ok();
            }
            SessionStatus::Destroy => {
                store.remove(&inner.id.to_string()).await.ok();
            }
            SessionStatus::Expire => {
                store.expire(&inner.id.to_string(), builder.expire_time).await.ok();
            }
        }
        let cookie = builder.build(inner.id.clone());
        if let Ok(cookie) = cookie.to_string().parse() {
            res.headers_mut().insert(SET_COOKIE, cookie);
        }
    }
}

pub trait SessionDepotExt {
    fn inner_session(&self) -> Option<Session>;
}

impl SessionDepotExt for Depot {

    fn inner_session(&self) -> Option<Session> {
        self.obtain::<Session>()
            .ok()
            .map(|x|x.clone())
    }
}