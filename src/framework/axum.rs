use crate::{SessionBuilder, SessionInner, SessionStatus, SessionStore};
use axum::{
    extract::Request,
    http::header::{HeaderValue, SET_COOKIE}
    ,
    response::Response,
};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use tower::{Layer, Service};

pub struct SessionMiddleware<S, Storage>
where
    S: Service<Request, Response = Response, Error = axum::http::Error> + Send + 'static,
    S::Future: Send + 'static,
    Storage: SessionStore + 'static,
{
    inner: Mutex<S>,
    builder: Arc<SessionBuilder>,
    store: Rc<Arc<Storage>>,
}

impl<S,Storage> Service<Request> for SessionMiddleware<S,Storage>
where
    Storage: SessionStore + 'static,
    S: Service<Request, Response = Response, Error = axum::http::Error> + Send + 'static + std::marker::Sync,
    S::Future: Send + 'static,
{
    type Response = Response;
    type Error = http::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.lock()
            .unwrap()
            .poll_ready(cx)
    }

    fn call(&mut self, mut req: Request) -> Self::Future {
        let store = self.store.clone();
        let builder = self.builder.clone();
        let session_key = req.headers()
            .get("Cookie")
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.split(';').find(|c| c.contains(&builder.key)))
            .map(|s| s.to_string());
        let mut inner = self.inner.lock().unwrap();
        Box::pin(async move {
            let session_inner = match session_key {
                Some(id) => {
                    if let Ok(data) = store.get(&id).await {
                        SessionInner {
                            session_id: id,
                            status: SessionStatus::UnChange,
                            data,
                        }
                    } else {
                        SessionInner {
                            session_id: id,
                            status: SessionStatus::UnChange,
                            data: HashMap::new(),
                        }
                    }
                },
                None => {
                    let new_id = builder.rand_key.generate();
                    SessionInner {
                        session_id: new_id,
                        status: SessionStatus::Change,
                        data: HashMap::new(),
                    }
                }
            };

            req.extensions_mut().insert(Arc::new(Mutex::new(session_inner)));
            let inner_service = inner.call(req);
            let mut res = inner_service.await?;

            if let Some(inner_arc) = res.extensions().get::<Arc<Mutex<SessionInner>>>() {
                let inner = inner_arc.lock().unwrap();
                match inner.status {
                    SessionStatus::UnChange => {
                        if builder.auto_expire {
                            store.expire(&inner.session_id, builder.expire_time).await.ok();
                        }
                    }
                    SessionStatus::Change => {
                        store.set(&inner.session_id, inner.data.clone()).await.ok();
                        store.expire(&inner.session_id, builder.expire_time).await.ok();
                    }
                    SessionStatus::Clear | SessionStatus::Destroy => {
                        store.remove(&inner.session_id).await.ok();
                    }
                    SessionStatus::Expire => {
                        store.expire(&inner.session_id, builder.expire_time).await.ok();
                    }
                }

                let cookie_value = format!("{}={}; Path=/", builder.key, inner.session_id);
                res.headers_mut().insert(SET_COOKIE, HeaderValue::from_str(&cookie_value).unwrap());
            }
            Ok(res)
        })
    }
}

pub struct SessionMiddlewareLayer<Storage>
where
    Storage: SessionStore + 'static,
{
    builder: Arc<SessionBuilder>,
    store: Rc<Arc<Storage>>,
}

impl <Storage>SessionMiddlewareLayer<Storage>
where
    Storage: SessionStore + 'static,
{
    pub fn new(builder: SessionBuilder, store: Storage) -> Self {
        Self {
            builder: Arc::new(builder),
            store: Rc::new(Arc::new(store)),
        }
    }
}

impl<S,Storage> Layer<S> for SessionMiddlewareLayer<Storage>
where
    Storage: SessionStore + 'static,
    S: Service<Request, Response = Response, Error = axum::http::Error> + Send + 'static + std::marker::Sync,
    S::Future: Send + 'static,
{
    type Service = SessionMiddleware<S, Storage>;

    fn layer(&self, inner: S) -> Self::Service {
        SessionMiddleware {
            inner,
            builder: self.builder.clone(),
            store: self.store.clone(),
        }
    }
}
