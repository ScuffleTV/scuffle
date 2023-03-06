use std::{
    future,
    str::FromStr,
    sync::{Arc, Weak},
};

use arc_swap::ArcSwap;
use async_graphql::{
    http::{WebSocketProtocols, WsMessage},
    Data,
};
use common::types::session;
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
    api::error::{Result, ResultExt, RouteError},
    global::GlobalState,
};

use super::{GqlContext, MySchema};

async fn websocket_handler(
    ws: HyperWebsocket,
    schema: MySchema,
    global: Arc<GlobalState>,
    protocol: WebSocketProtocols,
    context: Arc<GqlContext>,
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

    let mut data = Data::default();

    data.insert(context);
    data.insert(global.clone());

    let stream = async_graphql::http::WebSocket::new(schema, input, protocol)
        .connection_data(data)
        .map(|msg| match msg {
            WsMessage::Text(text) => Message::Text(text),
            WsMessage::Close(code, status) => Message::Close(Some(CloseFrame {
                code: code.into(),
                reason: status.into(),
            })),
        })
        .map(Ok);

    // TODO: Gracefully shutdown the stream forward.
    //  This is interesting since when we shutdown we interrupt the stream forward rather then waiting for the stream to finish.
    select! {
        _ = stream.forward(&mut tx) => {}
        _ = global.ctx.done() => {
            tx.send(Message::Close(Some(CloseFrame { code: CloseCode::Restart, reason: "server is restarting".into() }))).await.ok();
        }
    }
}

pub async fn graphql_handler(mut req: Request<Body>) -> Result<Response<Body>> {
    if req.method() == hyper::Method::OPTIONS {
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
        .data::<MySchema>()
        .expect("failed to get schema")
        .clone();

    let global = req
        .data::<Weak<GlobalState>>()
        .and_then(|w| w.upgrade())
        .ok_or((
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to get global state",
        ))?;

    let mut session = GqlContext {
        is_websocket: false,
        session: ArcSwap::from_pointee(req.context::<session::Model>()),
    };

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
            .ok_or((StatusCode::BAD_REQUEST, "Invalid websocket protocol"))?;

        let (mut response, websocket) = hyper_tungstenite::upgrade(&mut req, None)
            .map_err_route((StatusCode::BAD_REQUEST, "Failed to upgrade to websocket"))?;

        session.is_websocket = true;

        response.headers_mut().insert(
            header::SEC_WEBSOCKET_PROTOCOL,
            protocol
                .sec_websocket_protocol()
                .parse()
                .expect("failed to set websocket protocol"),
        );

        tokio::spawn(websocket_handler(
            websocket,
            schema,
            global,
            protocol,
            Arc::new(session),
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
                .and_then(|f| f.ok())
                .ok_or((StatusCode::BAD_REQUEST, "Invalid request body"))?;

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
            .map_err_route((StatusCode::BAD_REQUEST, "Invalid request body"))?
        }
        hyper::Method::GET => {
            let query = req.uri().query().unwrap_or("");

            async_graphql::http::parse_query_string(query)
                .map_err_route((StatusCode::BAD_REQUEST, "Invalid query string"))?
        }
        _ => {
            return Err(RouteError::from((
                StatusCode::METHOD_NOT_ALLOWED,
                "Invalid request method",
            )))
        }
    }
    .data(global)
    .data(Arc::new(session));

    let response = schema.execute(request).await;

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
