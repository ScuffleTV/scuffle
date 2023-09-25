use hyper::{server::conn::Http, Body, Response, StatusCode};
use routerify::{RequestInfo, RequestServiceBuilder, Router};
use serde_json::json;
use std::{io, sync::Arc};
use tokio::{net::TcpSocket, select};

use crate::{api::macros::make_response, global::GlobalState};

use self::error::{ApiError, ApiErrorInterface};

pub mod error;
pub mod ext;
pub mod macros;
pub mod middleware;
pub mod v1;

async fn error_handler(
    err: Box<(dyn std::error::Error + Send + Sync + 'static)>,
    info: RequestInfo,
) -> Response<Body> {
    match err.downcast::<ApiErrorInterface>() {
        Ok(err) => {
            let location = err.location();

            err.span().in_scope(|| match err.error() {
                ApiError::InternalServerError(_) => {
                    tracing::error!(
                        location = location.to_string(),
                        error = err.to_string(),
                        "http error"
                    );
                }
                _ => {
                    tracing::debug!(
                        location = location.to_string(),
                        error = err.to_string(),
                        "http error"
                    );
                }
            });

            err.response()
        }
        Err(err) => {
            tracing::error!(error = err, info = ?info, "unhandled http error");
            make_response!(
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({ "message": "Internal Server Error", "success": false })
            )
        }
    }
}

pub fn routes(global: &Arc<GlobalState>) -> Router<Body, ApiErrorInterface> {
    let weak = Arc::downgrade(global);
    Router::builder()
        .data(weak)
        // These response header middlewares lets us add headers to the response from the request handlers
        .middleware(middleware::response_headers::pre_flight_middleware(global))
        .middleware(middleware::response_headers::post_flight_middleware(global))
        // Our error handler
        .err_handler_with_info(error_handler)
        // The CORS middleware adds the CORS headers to the response
        .middleware(middleware::cors::cors_middleware(global))
        // The auth middleware checks the Authorization header, and if it's valid, it adds the user to the request extensions
        // This way, we can access the user in the handlers, this does not fail the request if the token is invalid or not present.
        .middleware(middleware::auth::auth_middleware(global))
        .scope("/v1", v1::routes(global))
        .build()
        .expect("failed to build router")
}

pub async fn run(global: Arc<GlobalState>) -> io::Result<()> {
    tracing::info!("Listening on {}", global.config.api.bind_address);
    let socket = if global.config.api.bind_address.is_ipv6() {
        TcpSocket::new_v6()?
    } else {
        TcpSocket::new_v4()?
    };

    socket.set_reuseaddr(true)?;
    socket.set_reuseport(true)?;
    socket.bind(global.config.api.bind_address)?;
    let listener = socket.listen(1024)?;

    // The reason we use a Weak reference to the global state is because we don't want to block the shutdown
    // When a keep-alive connection is open, the request service will still be alive, and will still be holding a reference to the global state
    // If we used an Arc, the global state would never be dropped, and the shutdown would never complete
    // By using a Weak reference, we can check if the global state is still alive, and if it isn't, we can stop accepting new connections
    let request_service =
        RequestServiceBuilder::new(routes(&global)).expect("failed to build request service");

    loop {
        select! {
            _ = global.ctx.done() => {
                return Ok(());
            },
            r = listener.accept() => {
                let (socket, addr) = r?;
                tracing::debug!("Accepted connection from {}", addr);

                tokio::spawn(Http::new().serve_connection(
                    socket,
                    request_service.build(addr),
                ).with_upgrades());
            },
        }
    }
}
