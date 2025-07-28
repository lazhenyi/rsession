use rsession::RandKey;
use rsession::framework::salvo::{SalvoSessionMiddleware, SessionDepotExt};
use rsession::redis::RedisSessionStorage;
use salvo::prelude::*;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();
    let redis = deadpool_redis::Config::from_url("redis://192.168.22.129:6379")
        .create_pool(Some(deadpool_redis::Runtime::Tokio1))
        .unwrap();

    let session = rsession::SessionBuilder::default();
    let mut store = RedisSessionStorage::new(redis, RandKey::UuidV7);
    store.set_prefix("actix_session:");
    let acceptor = TcpListener::new("0.0.0.0:8080").bind().await;
    let router = Router::new()
        .hoop(SalvoSessionMiddleware::new(session.clone(), store.clone()))
        .get(index);
    Server::new(acceptor).serve(router).await;
}

#[handler]
async fn index(_req: &mut Request, depot: &mut Depot, res: &mut Response, _ctrl: &mut FlowCtrl) {
    let session = depot.inner_session().unwrap();
    let r = if session.get::<i32>("count").is_err() {
        session.set("count", 1).ok();
        format!("count: {:?}", session.get::<i32>("count").unwrap())
    } else {
        let count = session.get::<i32>("count").unwrap();
        session.set("count", count + 1).ok();
        format!("count: {:?}", session.get::<i32>("count").unwrap())
    };
    res.body(r);
}
