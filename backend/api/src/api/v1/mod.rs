use std::convert::Infallible;

use hyper::Body;
use routerify::Router;

mod health;
mod users;

pub fn routes() -> Router<Body, Infallible> {
    Router::builder()
        .scope("/health", health::routes())
        .scope("/users", users::routes())
        .build()
        .unwrap()
}
