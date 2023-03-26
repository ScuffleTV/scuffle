use std::collections::HashMap;

use js_sys::ArrayBuffer;
use tokio::sync::mpsc;
use wasm_bindgen::{prelude::Closure, JsCast, JsValue};
use web_sys::{XmlHttpRequest, XmlHttpRequestResponseType};

pub struct FetchRequest {
    url: String,
    headers: HashMap<String, String>,
    method: String,
    timeout: Option<u32>,
}
pub struct InflightRequest {
    xhr: XmlHttpRequest,
}

#[derive(Debug, Default, Clone)]
pub struct Metrics {
    pub start_time: f64,
    pub ttfb: f64,
    pub download_time: f64,
    pub download_size: u32,
    pub size: u32,
    pub cached: bool,
}

impl FetchRequest {
    pub fn new(method: impl ToString, url: impl ToString) -> Self {
        Self {
            url: url.to_string(),
            headers: HashMap::new(),
            method: method.to_string(),
            timeout: None,
        }
    }

    pub fn header(mut self, key: impl ToString, value: impl ToString) -> Self {
        self.headers.insert(key.to_string(), value.to_string());
        self
    }

    pub fn set_timeout(mut self, timeout: u32) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn start(&self) -> Result<InflightRequest, JsValue> {
        let req = XmlHttpRequest::new()?;

        req.set_response_type(XmlHttpRequestResponseType::Arraybuffer);

        if let Some(timeout) = self.timeout {
            req.set_timeout(timeout);
        }

        req.open(&self.method, &self.url)?;

        for (key, value) in &self.headers {
            req.set_request_header(key, value)?;
        }

        req.send()?;

        Ok(InflightRequest { xhr: req })
    }
}

impl InflightRequest {
    pub async fn wait_result(&self) -> Result<Vec<u8>, JsValue> {
        let (tx, mut rx) = mpsc::channel(1);

        let closure = Closure::<dyn FnMut()>::new(move || {
            tx.try_send(()).ok();
        });

        self.xhr
            .set_onloadend(Some(closure.as_ref().unchecked_ref()));

        rx.recv().await;

        self.xhr.set_onloadend(None);
        drop(closure);

        let result = self.result()?;

        result.ok_or(JsValue::from_str("no result from request"))
    }

    pub fn is_done(&self) -> bool {
        self.xhr.ready_state() == XmlHttpRequest::DONE
    }

    pub fn result(&self) -> Result<Option<Vec<u8>>, JsValue> {
        if !self.is_done() {
            return Ok(None);
        }

        if self.xhr.status()? >= 399 {
            return Err(JsValue::from_str(
                format!("HTTP Error: {}", self.xhr.status()?).as_str(),
            ));
        }

        let resp = self.xhr.response()?;
        let Some(buf) = resp.dyn_ref::<ArrayBuffer>() else {
            return Err(resp);
        };

        Ok(Some(js_sys::Uint8Array::new(buf).to_vec()))
    }

    pub fn abort(&self) {
        self.xhr.abort().ok();
    }
}

impl Drop for InflightRequest {
    fn drop(&mut self) {
        self.xhr.set_onloadend(None);
        self.abort();
    }
}
