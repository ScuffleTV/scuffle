use std::{future, str::FromStr, sync::Arc};

use async_graphql::{
    http::{WebSocketProtocols, WsMessage},
    Data,
};
use common::http::ext::*;
use futures_util::{SinkExt, StreamExt};
use hyper::{body::HttpBody, header, Body, Request, Response, StatusCode};
use hyper_tungstenite::{
    tungstenite::{
        protocol::{frame::coding::CloseCode, CloseFrame},
        Message,
    },
    HyperWebsocket,
};
use routerify::prelude::RequestExt;
use serde_json::json;
use tokio::select;

use crate::{
    api::{
        error::Result,
        jwt::JwtState,
        middleware::auth::AuthError,
        request_context::{AuthData, RequestContext},
    },
    global::ApiGlobal,
};

use super::{error::GqlError, ext::RequestExt as _, MySchema};

async fn websocket_handler<G: ApiGlobal>(
    ws: HyperWebsocket,
    schema: MySchema<G>,
    global: Arc<G>,
    protocol: WebSocketProtocols,
    request_context: RequestContext,
) {
    let ws = match ws.await {
        Ok(ws) => ws,
        Err(e) => {
            tracing::error!("Failed to upgrade websocket request: {}", e);
            return;
        }
    };

    let (mut tx, rx) = ws.split();

    let input = rx
        .take_while(|res| future::ready(res.is_ok()))
        .map(Result::unwrap) // Safe because we check if its ok above
        .filter_map(|msg| {
            if let Message::Text(_) | Message::Binary(_) = msg {
                future::ready(Some(msg))
            } else {
                future::ready(None)
            }
        })
        .map(Message::into_data);

    let data = Data::default()
        .provide_context(request_context.clone())
        .provide_global(global.clone());

    let stream = {
        let global = global.clone();

        async_graphql::http::WebSocket::new(schema, input, protocol)
            .on_connection_init(|params| async move {
                // if the token is provided in the connection params we use that
                // and we fail if the token is invalid, there doesnt seem to be a way to return an error
                // from the connection_init callback that does not close the connection.
                // Or a way to tell the client that the token they provided is not valid.
                if let Some(token) = params.get("sessionToken").and_then(|v| v.as_str()) {
                    // We silently ignore invalid tokens since we don't want to force the user to login
                    // if the token is invalid when they make a request which requires authentication, it will fail.
                    let Some(jwt) = JwtState::verify(&global, token) else {
                        return Err(GqlError::Auth(AuthError::InvalidToken).into());
                    };

                    if let Ok(data) = AuthData::from_session_id(&global, jwt.session_id).await {
                        request_context.set_auth(data).await;
                    }
                }

                Ok(data)
            })
            .map(|msg| match msg {
                WsMessage::Text(text) => Message::Text(text),
                WsMessage::Close(code, status) => Message::Close(Some(CloseFrame {
                    code: code.into(),
                    reason: status.into(),
                })),
            })
            .map(Ok)
    };

    // TODO: Gracefully shutdown the stream forward.
    //  This is interesting since when we shutdown we interrupt the stream forward rather then waiting for the stream to finish.
    select! {
        _ = stream.forward(&mut tx) => {}
        _ = global.ctx().done() => {
            tx.send(Message::Close(Some(CloseFrame { code: CloseCode::Restart, reason: "server is restarting".into() }))).await.ok();
        }
    }
}

pub async fn graphql_handler<G: ApiGlobal>(mut req: Request<Body>) -> Result<Response<Body>> {
    if req.method() == hyper::Method::OPTIONS {
        // TODO: Why this? We have cors middleware
        return Ok(hyper::Response::builder()
            .status(StatusCode::OK)
            .header("Access-Control-Allow-Origin", "*")
            .header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
            .header(
                "Access-Control-Allow-Headers",
                "Content-Type, Authorization",
            )
            .header("Access-Control-Max-Age", "86400")
            .body(Body::empty())
            .expect("failed to build response"));
    }

    let schema = req
        .data::<MySchema<G>>()
        .expect("failed to get schema")
        .clone();

    let global = req.get_global::<G>()?;

    let context: RequestContext = req.context().expect("missing request context");

    // We need to check if this is a websocket upgrade request.
    // If it is, we need to upgrade the request to a websocket request.
    if hyper_tungstenite::is_upgrade_request(&req) {
        let protocol = req
            .headers()
            .get(header::SEC_WEBSOCKET_PROTOCOL)
            .and_then(|val| val.to_str().ok())
            .and_then(|protocols| {
                protocols
                    .split(',')
                    .find_map(|p| WebSocketProtocols::from_str(p.trim()).ok())
            })
            .map_err_route((StatusCode::BAD_REQUEST, "invalid websocket protocol"))?;

        let (mut response, websocket) = hyper_tungstenite::upgrade(&mut req, None)
            .map_err_route("failed to upgrade request")?;

        response.headers_mut().insert(
            header::SEC_WEBSOCKET_PROTOCOL,
            protocol
                .sec_websocket_protocol()
                .parse()
                .expect("failed to set websocket protocol"),
        );

        tokio::spawn(websocket_handler(
            websocket, schema, global, protocol, context,
        ));

        return Ok(response);
    }

    // We need to parse the request body into a GraphQL request.
    // If the request is a post request, we need to parse the body as a GraphQL request.
    // If the request is a get request, we need to parse the query string as a GraphQL request.
    let request = match *req.method() {
        hyper::Method::POST => {
            let body = req
                .body_mut()
                .data()
                .await
                .map_err_route((StatusCode::BAD_REQUEST, "missing request body"))?
                .map_err_route((StatusCode::BAD_REQUEST, "failed to read body"))?;

            let content_type = req
                .headers()
                .get("content-type")
                .and_then(|val| val.to_str().ok());

            async_graphql::http::receive_body(
                content_type,
                body.to_vec().as_slice(),
                Default::default(),
            )
            .await
            .map_err_route((StatusCode::BAD_REQUEST, "failed to parse body"))?
        }
        hyper::Method::GET => {
            let query = req
                .uri()
                .query()
                .map_err_route((StatusCode::BAD_REQUEST, "missing query string"))?;
            async_graphql::http::parse_query_string(query)
                .map_err_route((StatusCode::BAD_REQUEST, "failed to parse query string"))?
        }
        _ => {
            return Err((StatusCode::METHOD_NOT_ALLOWED, "method not allowed").into());
        }
    }
    .provide_global(global)
    .provide_context(context);

    let response = schema.execute(request).await;

    // TODO: Why all those headers? We have a CORS middleware
    let mut resp = Response::builder()
        .status(StatusCode::OK)
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
        .header(
            "Access-Control-Allow-Headers",
            "Content-Type, Authorization",
        )
        .header("Access-Control-Max-Age", "86400")
        .header("Content-Type", "application/json")
        .body(Body::from(
            json!({
                "data": response.data,
                "errors": if response.errors.is_empty() {
                    None
                } else {
                    Some(response.errors)
                },
                "extensions": response.extensions,
            })
            .to_string(),
        ))
        .expect("failed to build response");

    (&response.http_headers)
        .into_iter()
        .for_each(|(key, value)| {
            resp.headers_mut().insert(key, value.clone());
        });

    Ok(resp)
}
