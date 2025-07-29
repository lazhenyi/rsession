use crate::{Session, SessionBuilder, SessionInner, SessionStatus, SessionStore};
use actix_web::body::MessageBody;
use actix_web::dev::{Payload, Service, ServiceRequest, ServiceResponse, Transform, forward_ready};
use actix_web::http::header::SET_COOKIE;
use actix_web::{FromRequest, HttpMessage, HttpRequest};
use std::cell::RefCell;
use std::future::{Ready, ready};
use std::pin::Pin;
use std::rc::Rc;

/// Actix-web middleware for session management
///
/// This middleware integrates session functionality into Actix-web applications,
/// handling session creation, storage operations, and cookie management.
///
/// # Type Parameters
/// * `T` - The session storage backend implementing SessionStore
#[derive(Clone)]
pub struct ActixSessionMiddleware<T>
where
    T: SessionStore,
{
    builder: Rc<SessionBuilder>,
    store: Rc<Box<T>>,
}

impl<T> ActixSessionMiddleware<T>
where
    T: SessionStore,
{
    /// Creates a new ActixSessionMiddleware
    ///
    /// # Arguments
    /// * `builder` - Session configuration builder with cookie/session settings
    /// * `store` - Session storage backend implementation
    pub fn new(builder: SessionBuilder, store: T) -> Self {
        Self {
            builder: Rc::new(builder),
            store: Rc::new(Box::new(store)),
        }
    }
}

/// Actix-web Transform implementation for session middleware
///
/// Enables the middleware to be integrated into the Actix-web service chain,
/// providing session management capabilities to the application.
impl<S, B, Store> Transform<S, ServiceRequest> for ActixSessionMiddleware<Store>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error> + 'static,
    S::Future: 'static,
    B: MessageBody + 'static,
    Store: SessionStore + 'static,
{
    type Response = ServiceResponse<B>;
    type Error = actix_web::Error;
    type Transform = ActixInnerSessionMiddleware<S, Store>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(ActixInnerSessionMiddleware {
            service: Rc::new(service),
            builder: Rc::clone(&self.builder),
            store: Rc::clone(&self.store),
        }))
    }
}

/// Inner Actix-web service middleware handling session processing
///
/// This struct implements the actual service logic for session management,
/// processing requests to load sessions and responses to persist changes.
#[derive(Clone)]
pub struct ActixInnerSessionMiddleware<S, Store>
where
    Store: SessionStore + 'static,
{
    builder: Rc<SessionBuilder>,
    store: Rc<Box<Store>>,
    service: Rc<S>,
}
/// Service implementation for Actix session middleware
///
/// Processes incoming requests to load existing sessions or create new ones,
/// and handles outgoing responses to persist session changes to storage and
/// update client cookies.
impl<S, B, Store> Service<ServiceRequest> for ActixInnerSessionMiddleware<S, Store>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error> + 'static,
    S::Future: 'static,
    B: MessageBody + 'static,
    Store: SessionStore + 'static,
{
    type Response = ServiceResponse<B>;
    type Error = actix_web::Error;
    #[allow(clippy::type_complexity)]
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    forward_ready!(service);
    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = Rc::clone(&self.service);
        let store = Rc::clone(&self.store);
        let builder = Rc::clone(&self.builder);
        let _ = req.app_data().insert(&builder.clone());
        Box::pin(async move {
            let session_key = req.cookie(&builder.key).map(|x| x.value().to_string());
            if let Some(session_key) = session_key {
                if let Ok(inner) = store.get(&session_key).await {
                    store.remove(&session_key).await.ok();
                    req.extensions_mut().insert(Rc::new(RefCell::new(inner)));
                } else {
                    req.extensions_mut()
                        .insert(Rc::new(RefCell::new(SessionInner::new(session_key))));
                }
            } else {
                req.extensions_mut()
                    .insert(Rc::new(RefCell::new(SessionInner::new(
                        builder.rand_key.generate(),
                    ))));
            }
            let mut res = service.call(req).await?;
            let inner = res
                .request()
                .extensions()
                .get::<Rc<RefCell<SessionInner>>>()
                .map(|x| x.clone());
            if let Some(status) = inner {
                let inner = status.borrow();
                match inner.status {
                    SessionStatus::UnChange => {
                        store.set(&inner.id.to_string(), inner.clone()).await.ok();
                        if builder.auto_expire {
                            store
                                .expire(&inner.id.to_string(), builder.expire_time)
                                .await
                                .ok();
                        }}
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
            Ok(res)
        })
    }
}

/// Actix-web FromRequest implementation for Session
///
/// Enables session extraction from Actix-web requests using the `Session` extractor
/// in route handlers.
///
/// # Example
/// ```rust
/// use actix_web::Responder;
/// use rsession::Session;
///
///  async fn handler(session: Session) -> impl Responder {
///     session.set("user_id", 123).unwrap();
///     "Session updated"
/// }
/// ```
impl FromRequest for Session {
    type Error = actix_web::Error;
    type Future = Ready<Result<Session, actix_web::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let inner = match req.extensions().get::<Rc<RefCell<SessionInner>>>() {
            Some(x) => x.clone(),
            None => {
                let builder = match req.app_data::<Rc<SessionBuilder>>() {
                    Some(x) => x.clone(),
                    None => {
                        return ready(Err(actix_web::error::ErrorInternalServerError(
                            "session config not found".to_string(),
                        )));
                    }
                };
                Rc::new(RefCell::new(SessionInner::new(builder.rand_key.generate())))
            }
        };
        ready(Ok(Session::new(inner)))
    }
}
