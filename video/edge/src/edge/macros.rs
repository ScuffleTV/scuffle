macro_rules! make_response {
    ($status:expr, $body:expr) => {
        hyper::Response::builder()
            .status($status)
            .header("Content-Type", "application/json")
            .body(Body::from($body.to_string()))
            .expect("failed to build response")
    };
}

pub(super) use make_response;
