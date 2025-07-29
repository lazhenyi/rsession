use actix_web::{App, HttpServer};
use rsession::framework::actix::ActixSessionMiddleware;
use rsession::redis::RedisSessionStorage;
use rsession::{RandKey, Session};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();
    HttpServer::new(move || {
        let redis = deadpool_redis::Config::from_url("redis://192.168.22.129:6379")
            .create_pool(Some(deadpool_redis::Runtime::Tokio1))
            .unwrap();
        let session = rsession::SessionBuilder::default();
        let mut store = RedisSessionStorage::new(redis, RandKey::RandomSha256(128));
        store.set_prefix("actix_test_session:");
        App::new()
            .wrap(ActixSessionMiddleware::new(session.clone(), store.clone()))
            .route("/", actix_web::web::get().to(index))
    })
    .bind("127.0.0.1:3080")
    .unwrap()
    .run()
    .await
    .unwrap();
}

async fn index(session: Session) -> String {
    return if session.get::<i32>("count").is_err() {
        session.set("count", 1).ok();
        format!("count: {:?}", session.get::<i32>("count").unwrap())
    } else {
        let count = session.get::<i32>("count").unwrap();
        session.set("count", count + 1).ok();
        format!("count: {:?}", session.get::<i32>("count").unwrap())
    };
}
