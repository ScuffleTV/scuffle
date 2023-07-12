use std::{cell::RefCell, collections::HashMap, rc::Rc};

use gloo_timers::future::TimeoutFuture;
use js_sys::ArrayBuffer;
use tokio::sync::mpsc;
use url::Url;
use wasm_bindgen::{prelude::Closure, JsCast, JsValue};
use web_sys::{
    PerformanceObserver, PerformanceObserverEntryList, PerformanceObserverInit,
    PerformanceResourceTiming, XmlHttpRequest, XmlHttpRequestResponseType,
};

use super::util::{register_events, Holder};

thread_local!(static PERFORMANCE_OBSERVER: PerformanceObserverHolder = PerformanceObserverHolder::new());

struct PerformanceObserverHolder {
    _obs: PerformanceObserver,
    _cb: Closure<dyn FnMut(PerformanceObserverEntryList)>,
    entries: Rc<RefCell<Vec<PerformanceResourceTiming>>>,
}

impl PerformanceObserverHolder {
    fn new() -> Self {
        let entries = Rc::new(RefCell::new(Vec::new()));

        tracing::info!("creating performance observer");

        let cb = {
            let entries = entries.clone();
            Closure::<dyn FnMut(_)>::new(move |items: PerformanceObserverEntryList| {
                let mut entries = entries.borrow_mut();
                entries.extend(
                    items
                        .get_entries()
                        .iter()
                        .map(|i| i.unchecked_into::<PerformanceResourceTiming>()),
                );

                let size = entries.len();
                if size > 250 {
                    entries.drain(0..size - 250);
                }
            })
        };

        let obs = PerformanceObserver::new(cb.as_ref().unchecked_ref()).unwrap();

        obs.observe(&PerformanceObserverInit::new(&js_sys::Array::of1(
            &"resource".into(),
        )));

        Self {
            _obs: obs,
            entries,
            _cb: cb,
        }
    }

    fn get_entries_by_name(&self, name: &str) -> Vec<PerformanceResourceTiming> {
        let entries = self.entries.borrow();
        let entries = entries
            .iter()
            .filter(|e| e.name() == name)
            .cloned()
            .collect();

        entries
    }
}

pub struct FetchRequest {
    url: Url,
    headers: HashMap<String, String>,
    method: String,
    timeout: Option<u32>,
}
pub struct InflightRequest {
    url: Url,
    xhr: Holder<XmlHttpRequest>,
    rx: mpsc::Receiver<()>,
}

#[derive(Debug, Default, Clone)]
pub struct Metrics {
    pub start_time: f64,
    pub ttfb: f64,
    pub download_time: f64,
    pub download_size: u32,
}

impl FetchRequest {
    pub fn new(method: &str, url: Url) -> Self {
        Self {
            url,
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
        // Make sure the performance observer is initialized
        PERFORMANCE_OBSERVER.with(|_| {});

        let req = XmlHttpRequest::new()?;

        req.set_response_type(XmlHttpRequestResponseType::Arraybuffer);

        if let Some(timeout) = self.timeout {
            req.set_timeout(timeout);
        }

        req.open(&self.method, self.url.as_str())?;

        for (key, value) in &self.headers {
            req.set_request_header(key, value)?;
        }

        req.send()?;

        // We need a large enough buffer to hold all events that can be fired
        // This is becasue we might read the events until the request is done
        let (tx, rx) = mpsc::channel(4);

        let cb = register_events!(req, {
            "loadend" => {
                move |_| {
                    if tx.try_send(()).is_err() {
                        tracing::warn!("fetch event queue full");
                    }
                }
            },
        });

        let xhr = Holder::new(req, cb);

        Ok(InflightRequest {
            url: self.url.clone(),
            xhr,
            rx,
        })
    }

    pub fn url(&self) -> &Url {
        &self.url
    }
}

impl InflightRequest {
    pub async fn wait_result(&mut self) -> Result<Vec<u8>, JsValue> {
        self.rx.recv().await;

        while !self.is_done() {
            TimeoutFuture::new(0).await;
        }

        let result = self.result()?;

        result.ok_or(JsValue::from_str("no result from request"))
    }

    pub fn is_done(&self) -> bool {
        self.xhr.ready_state() == XmlHttpRequest::DONE && self.metrics().is_some()
    }

    pub fn metrics(&self) -> Option<Metrics> {
        let entity = PERFORMANCE_OBSERVER.with(|p| p.get_entries_by_name(self.url.as_str()));

        let Some(entity) = entity.first() else {
            return None;
        };

        let start_time = entity.fetch_start();
        let ttfb = entity.response_start() - start_time;
        let download_time = entity.response_end() - entity.response_start();
        let download_size = entity.transfer_size() as u32;

        Some(Metrics {
            start_time,
            ttfb,
            download_size,
            download_time,
        })
    }

    pub fn result(&mut self) -> Result<Option<Vec<u8>>, JsValue> {
        if !self.is_done() {
            return Ok(None);
        }

        if self.xhr.status()? >= 399 {
            return Err(JsValue::from_str(
                format!("HTTP Error: {}", self.xhr.status()?).as_str(),
            ));
        }

        let resp = self.xhr.response()?;
        if resp.is_null() {
            return Err("request aborted or no response".into());
        }

        let Some(buf) = resp.dyn_ref::<ArrayBuffer>() else {
            return Err("response is not an ArrayBuffer".into());
        };

        Ok(Some(js_sys::Uint8Array::new(buf).to_vec()))
    }

    pub fn abort(&self) {
        self.xhr.abort().ok();
    }

    pub fn url(&self) -> &Url {
        &self.url
    }
}

impl Drop for InflightRequest {
    fn drop(&mut self) {
        self.xhr.set_onloadend(None);
        self.abort();
    }
}
