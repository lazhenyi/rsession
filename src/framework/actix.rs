use crate::{Session, SessionBuilder, SessionInner, SessionStatus, SessionStore};
use actix_web::body::MessageBody;
use actix_web::dev::{forward_ready, Payload, Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::http::header::{HeaderValue, SET_COOKIE};
use actix_web::{FromRequest, HttpMessage, HttpRequest};
use std::cell::RefCell;
use std::future::{ready, Ready};
use std::pin::Pin;
use std::rc::Rc;

#[derive(Clone)]
pub struct ActixSessionMiddleware<T>
where T: SessionStore
{
    builder: Rc<SessionBuilder>,
    store: Rc<Box<T>>,
}

impl <T> ActixSessionMiddleware<T>
where T: SessionStore
{
   pub fn new(builder: SessionBuilder, store: T) -> Self {
       Self {
           builder: Rc::new(builder),
           store: Rc::new(Box::new(store)),
       }
   }
}


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


#[derive(Clone)]
pub struct ActixInnerSessionMiddleware<S, Store>
where
    Store: SessionStore + 'static,
{
    builder: Rc<SessionBuilder>,
    store: Rc<Box<Store>>,
    service: Rc<S>,
}
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
            let session_key = req.cookie(&builder.key)
                .map(|x|x.value().to_string());
            if let Some(session_key) = session_key {
                if let Ok(session) = store.get(&session_key).await {
                    let inner = SessionInner{
                        session_id: session_key,
                        status: SessionStatus::UnChange,
                        data: session,
                    };
                    req.extensions_mut().insert(Rc::new(RefCell::new(inner)));
                } else {
                    req.extensions_mut().insert(Rc::new(RefCell::new(SessionInner::new(session_key))));
                }
            } else {
                req.extensions_mut().insert(Rc::new(RefCell::new(SessionInner::new(builder.rand_key.generate()))));
            }
            let mut res = service.call(req).await?;
            let inner = res.request().extensions().get::<Rc<RefCell<SessionInner>>>().map(|x|x.clone());
            if let Some(status) = inner {
                let inner = status.borrow();
                match inner.status {
                    SessionStatus::UnChange => {
                        if builder.auto_expire {
                            store.expire(&inner.session_id, builder.expire_time).await.ok();
                        }
                    }
                    SessionStatus::Change => {
                        store.remove(&inner.session_id).await.ok();
                        store.set(&inner.session_id, inner.data.clone()).await.ok();
                        store.expire(&inner.session_id, builder.expire_time).await.ok();
                    }
                    SessionStatus::Clear => {
                        store.remove(&inner.session_id).await.ok();
                    }
                    SessionStatus::Destroy => {
                        store.remove(&inner.session_id).await.ok();
                    }
                    SessionStatus::Expire => {
                        store.expire(&inner.session_id, builder.expire_time).await.ok();
                    }
                }
                let cookie = inner.builder(&builder);
                res.headers_mut().insert(SET_COOKIE, HeaderValue::from_str(&cookie.to_string()).unwrap());
            }
            Ok(res)
            
        })
    }
}


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
                            "session config not found".to_string()
                        )));
                    }
                };
                Rc::new(RefCell::new(SessionInner::new(builder.rand_key.generate())))
            },
        };
        ready(Ok(Session::new(inner)))
    }
}