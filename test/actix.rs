use actix_web::{App, HttpServer};
use rsession::actix::ActixSessionMiddleware;
use rsession::redis::RedisSessionStorage;
use rsession::Session;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();
    HttpServer::new(move || {
        let redis = deadpool_redis::Config::from_url("redis://192.168.100.6:6379")
            .create_pool(Some(deadpool_redis::Runtime::Tokio1))
            .unwrap();
        let store = RedisSessionStorage::new(redis);
        let session = rsession::SessionBuilder::default();
        App::new()
            .wrap(
                ActixSessionMiddleware::new(
                    session.clone(),
                    store.clone()
                )
            )
            .route("/", actix_web::web::get().to(index))
    })
        .bind("127.0.0.1:8080").unwrap()
        .run().await.unwrap();
}


async fn index(session: Session) -> String {
    return if session.get::<i32>("count").is_err() {
        session.set("count", 1).ok();
        format!("count: {:?}", session.get::<i32>("count").unwrap())
    } else {
        let count = session.get::<i32>("count").unwrap();
        session.set("count", count.unwrap_or(1) + 1).ok();
        format!("count: {:?}", session.get::<i32>("count").unwrap())
    }
}