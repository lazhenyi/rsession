//! Axum session management middleware
//!
//! This module provides Axum integration for session management using Tower middleware,
//! handling session creation, storage operations, and cookie management.
use crate::{Session, SessionBuilder, SessionInner, SessionStatus, SessionStore};
use axum::body::Body;
use axum::http::HeaderMap;
use axum::http::header::COOKIE;
use axum::{extract::Request, response::Response};
use cookie::{Cookie, CookieJar};
use futures::future::BoxFuture;
use http::header::SET_COOKIE;
use std::cell::RefCell;
use std::convert::Infallible;
use std::rc::Rc;
use std::sync::Arc;
use std::task::{Context, Poll};
use tower::{Layer, Service};

/// Axum session management middleware
///
/// This Tower Service implementation integrates session management into Axum applications,
/// handling session loading from storage, request processing, and response finalization.
///
/// # Type Parameters
/// * `S` - The inner service type
/// * `Storage` - The session storage backend implementing SessionStore
#[derive(Clone)]
pub struct AxumSessionMiddleware<S, Storage>
where
    S: Service<Request<Body>, Response = Response, Error = Infallible> + Send + 'static,
    S::Future: Send + 'static,
    Storage: SessionStore + 'static + Send + Sync + Clone,
{
    inner: S,
    builder: Arc<SessionBuilder>,
    store: Arc<Storage>,
}

/// Tower Service implementation for Axum session middleware
///
/// Processes incoming requests to load or create sessions, injects the session into
/// request extensions, and handles responses to persist session changes and update cookies.
impl<S, Storage> Service<Request<Body>> for AxumSessionMiddleware<S, Storage>
where
    Storage: SessionStore + 'static + Send + Sync + Clone,
    S: Service<Request<Body>, Response = Response, Error = Infallible> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = Response;
    type Error = Infallible;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<Body>) -> Self::Future {
        let store = self.store.clone();
        let not_ready_inner = self.inner.clone();
        let mut ready_inner = std::mem::replace(&mut self.inner, not_ready_inner);
        let builder = self.builder.clone();
        Box::pin(async move {
            let cookies = get_cookies(req.headers());
            let session_key = cookies.get(&builder.key);
            let session_inner = if let Some(session_key) = session_key {
                let session_key = session_key.value().to_string();
                if let Ok(inner) = store.get(&session_key).await {
                    inner
                } else {
                    SessionInner::new(builder.rand_key.generate())
                }
            } else {
                SessionInner::new(builder.rand_key.generate())
            };
            let session = Session::new(Rc::new(RefCell::new(session_inner)));
            req.extensions_mut().insert(session.clone());
            let future = ready_inner.call(req);
            let res = future.await;
            match res {
                Ok(mut res) => {
                    let inner = session.inner();
                    let cookie = builder.build(inner.id.clone());
                    if let Ok(cookie) = cookie.to_string().parse() {
                        res.headers_mut().insert(SET_COOKIE, cookie);
                    }
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
                    Ok(res)
                }
                Err(err) => Err(err),
            }
        })
    }
}

/// Tower Layer for Axum session middleware
///
/// Provides a configuration layer for adding session management to Axum applications
/// through the Tower middleware system.
///
/// # Type Parameters
/// * `Storage` - The session storage backend implementing SessionStore
#[derive(Clone)]
pub struct AxumSessionMiddlewareLayer<Storage>
where
    Storage: SessionStore + 'static,
{
    builder: Arc<SessionBuilder>,
    store: Arc<Storage>,
}

impl<Storage> AxumSessionMiddlewareLayer<Storage>
where
    Storage: SessionStore + 'static,
{
    /// Creates a new AxumSessionMiddlewareLayer
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

/// Tower Layer implementation for Axum session middleware
///
/// Applies the session middleware to an Axum service, enabling session management
/// for all routes wrapped by this layer.
impl<S, Storage> Layer<S> for AxumSessionMiddlewareLayer<Storage>
where
    Storage: SessionStore + 'static,
    S: Service<Request, Response = Response, Error = Infallible>
        + Send
        + 'static
        + std::marker::Sync,
    S::Future: Send + 'static,
{
    type Service = AxumSessionMiddleware<S, Storage>;

    fn layer(&self, inner: S) -> Self::Service {
        AxumSessionMiddleware {
            inner,
            builder: self.builder.clone(),
            store: self.store.clone(),
        }
    }
}

/// Parses cookies from request headers into a CookieJar
///
/// # Arguments
/// * `headers` - Request headers containing cookie information
///
/// # Returns
/// A CookieJar populated with cookies from the request
pub(crate) fn get_cookies(headers: &HeaderMap) -> CookieJar {
    let mut jar = CookieJar::new();
    let cookie_iter = headers
        .get_all(COOKIE)
        .into_iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(';'))
        .filter_map(|cookie| Cookie::parse(cookie.to_owned()).ok());
    for cookie in cookie_iter {
        jar.add_original(cookie);
    }
    jar
}

impl<S> axum::extract::FromRequest<S> for Session {
    type Rejection = (axum::http::status::StatusCode, &'static str);

    fn from_request(
        req: Request,
        _: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        async move {
            let inner = req.extensions().get::<Session>();
            if let Some(inner) = inner {
                return Ok(inner.clone());
            } else {
                Err((
                    axum::http::status::StatusCode::INTERNAL_SERVER_ERROR,
                    "session not found",
                ))
            }
        }
    }
}

// impl<S> axum::extract::FromRequestParts<S> for Session {
//     type Rejection = (axum::http::status::StatusCode, &'static str);
//     fn from_request_parts(parts: &mut Parts, _: &S) -> impl Future<Output=Result<Self, Self::Rejection>> + Send {
//         async move {
//             let inner = parts.extensions.get::<SessionInner>();
//             if let Some(inner) = inner {
//                 Ok(Session::new(Rc::new(RefCell::new(inner.clone()))))
//             } else {
//                 Err((
//                     axum::http::status::StatusCode::INTERNAL_SERVER_ERROR,
//                     "session not found",
//                 ))
//             }
//         }
//     }
// }
