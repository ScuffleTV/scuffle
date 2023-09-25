use std::sync::{Arc, Weak};

use hyper::{Body, Request};
use routerify::prelude::RequestExt as _;

use crate::global::GlobalState;

use super::error::{ApiError, Result};

pub trait RequestExt {
    fn get_global(&self) -> Result<Arc<GlobalState>>;
}

impl RequestExt for Request<Body> {
    fn get_global(&self) -> Result<Arc<GlobalState>> {
        let state = self
            .data::<Weak<GlobalState>>()
            .expect("global state not set")
            .upgrade()
            .ok_or(ApiError::InternalServerError(
                "failed to upgrade global state",
            ))?;
        Ok(state)
    }
}
