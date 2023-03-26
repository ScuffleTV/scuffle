use std::ops::{Deref, DerefMut};

use wasm_bindgen::JsCast;

type Cleanup = Box<dyn FnOnce(&web_sys::EventTarget)>;

pub struct Holder<T: JsCast> {
    inner: T,
    cleanup: Option<Cleanup>,
}

impl<T: JsCast> Holder<T> {
    pub fn new(inner: T, cleanup: Cleanup) -> Self {
        Self {
            inner,
            cleanup: Some(cleanup),
        }
    }
}

impl<T: JsCast> Deref for Holder<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T: JsCast> DerefMut for Holder<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<T: JsCast> Drop for Holder<T> {
    fn drop(&mut self) {
        if let Some(cleanup) = self.cleanup.take() {
            cleanup(self.inner.unchecked_ref());
        }
    }
}

macro_rules! register_events {
    ($ob:ident, {
        $(
            $($evt:literal)|+ => $body:expr
        ),* $(,)?
    }) => {
        {
            let mut handlers = std::collections::VecDeque::new();
            $(
                handlers.push_back((vec![$($evt.to_string()),+], Closure::<dyn FnMut(web_sys::Event)>::new($body)));
                $(
                    $ob.add_event_listener_with_callback($evt, handlers.back().unwrap().1.as_ref().unchecked_ref()).unwrap();
                )*
            )*

            Box::new(move |val: &web_sys::EventTarget| {
                handlers.drain(..).for_each(|(evts, cb)| {
                    for evt in evts {
                        val.remove_event_listener_with_callback(&evt, cb.as_ref().unchecked_ref()).unwrap();
                    }
                });
            }) as Box<dyn FnOnce(&web_sys::EventTarget)>
        }
    };
}

pub(super) use register_events;
