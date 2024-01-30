use std::pin::Pin;

use bytes::Bytes;
use futures_util::AsyncRead;

#[pin_project::pin_project]
pub struct AsyncReadBody<B: hyper::body::Body>(#[pin] B, Option<Bytes>);

#[pin_project::pin_project]
pub struct BodyStream<B: hyper::body::Body>(#[pin] B);

impl<B: hyper::body::Body<Data = Bytes, Error = E>, E: std::error::Error + Send + Sync + 'static> AsyncRead
	for AsyncReadBody<B>
{
	fn poll_read(
		self: Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
		buf: &mut [u8],
	) -> std::task::Poll<std::io::Result<usize>> {
		let this = self.project();
		if let Some(frame) = this.1.take() {
			if buf.len() >= frame.len() {
				buf[..frame.len()].copy_from_slice(&frame);
				return std::task::Poll::Ready(Ok(frame.len()));
			} else {
				buf.copy_from_slice(&frame[..buf.len()]);
				*this.1 = Some(frame.slice(buf.len()..));
				return std::task::Poll::Ready(Ok(buf.len()));
			}
		}

		match this.0.poll_frame(cx) {
			std::task::Poll::Ready(Some(Ok(frame))) => {
				let Ok(data) = frame.into_data() else {
					return std::task::Poll::Ready(Ok(0));
				};

				if buf.len() >= data.len() {
					buf[..data.len()].copy_from_slice(&data);
					std::task::Poll::Ready(Ok(data.len()))
				} else {
					buf.copy_from_slice(&data[..buf.len()]);
					*this.1 = Some(data.slice(buf.len()..));
					std::task::Poll::Ready(Ok(buf.len()))
				}
			}
			std::task::Poll::Ready(Some(Err(err))) => {
				std::task::Poll::Ready(Err(std::io::Error::new(std::io::ErrorKind::Other, err)))
			}
			std::task::Poll::Ready(None) => std::task::Poll::Ready(Ok(0)),
			std::task::Poll::Pending => std::task::Poll::Pending,
		}
	}
}

impl<B: hyper::body::Body<Data = Bytes>> futures_util::Stream for BodyStream<B> {
	type Item = Result<Bytes, B::Error>;

	fn poll_next(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<Self::Item>> {
		let this = self.project();
		match this.0.poll_frame(cx) {
			std::task::Poll::Ready(Some(Ok(data))) => {
				if let Ok(data) = data.into_data() {
					std::task::Poll::Ready(Some(Ok(data)))
				} else {
					std::task::Poll::Ready(None)
				}
			}
			std::task::Poll::Ready(Some(Err(err))) => std::task::Poll::Ready(Some(Err(err))),
			std::task::Poll::Ready(None) => std::task::Poll::Ready(None),
			std::task::Poll::Pending => std::task::Poll::Pending,
		}
	}
}

pub trait BodyExt: hyper::body::Body + Sized {
	fn into_async_read(self) -> AsyncReadBody<Self>;
	fn into_stream(self) -> BodyStream<Self>;
}

impl<B: hyper::body::Body> BodyExt for B {
	fn into_async_read(self) -> AsyncReadBody<Self> {
		AsyncReadBody(self, None)
	}

	fn into_stream(self) -> BodyStream<Self> {
		BodyStream(self)
	}
}
