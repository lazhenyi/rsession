use axum::response::IntoResponse;
use axum::routing::get;
use rsession::framework::axum::AxumSessionMiddlewareLayer;
use rsession::redis::RedisSessionStorage;
use rsession::{RandKey, Session};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();
    let redis = deadpool_redis::Config::from_url("redis://192.168.22.129:6379")
        .create_pool(Some(deadpool_redis::Runtime::Tokio1))
        .unwrap();

    let session = rsession::SessionBuilder::default();
    let mut store = RedisSessionStorage::new(redis, RandKey::UuidV7);
    store.set_prefix("actix_session:");
    let app =
        axum::Router::new()
            .route("/", get(index))
            .route_layer(AxumSessionMiddlewareLayer::new(
                session.clone(),
                store.clone(),
            ));
    axum::serve(
        TcpListener::bind("127.0.0.1:8080").await.unwrap(),
        app.into_make_service(),
    )
    .await
    .ok();
}

#[axum::debug_handler]
pub async fn index(session: Session) -> impl IntoResponse {
    return if session.get::<i32>("count").is_err() {
        session.set("count", 1).ok();
        format!("count: {:?}", session.get::<i32>("count").unwrap())
    } else {
        let count = session.get::<i32>("count").unwrap();
        session.set("count", count + 1).ok();
        format!("count: {:?}", session.get::<i32>("count").unwrap())
    };
}
