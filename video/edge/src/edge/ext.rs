use std::sync::{Arc, Weak};

use hyper::{Body, Request, StatusCode};
use routerify::prelude::RequestExt as _;

use crate::global::GlobalState;

use super::error::Result;

pub trait RequestExt {
    fn get_global(&self) -> Result<Arc<GlobalState>>;
}

impl RequestExt for Request<Body> {
    fn get_global(&self) -> Result<Arc<GlobalState>> {
        Ok(self
            .data::<Weak<GlobalState>>()
            .expect("global state not set")
            .upgrade()
            .ok_or((
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to upgrade global state",
            ))?)
    }
}
