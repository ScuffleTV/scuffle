use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use axum::body::Body;
use axum::extract::Request;
use axum::response::IntoResponse;
use bytes::{Buf, Bytes};
use futures::future::poll_fn;
use futures::Future;
use h3::error::{Code, ErrorLevel};
use h3::ext::Protocol;
use h3::server::{Builder, RequestStream};
use h3_quinn::{BidiStream, RecvStream, SendStream};
use http::Response;
use http_body::Body as HttpBody;
use quinn::Connecting;
#[cfg(not(feature = "runtime"))]
use tokio::spawn;
use tracing::Instrument;

use super::{Backend, IncomingConnection, MakeService, ServiceHandler, SocketKind};
use crate::context::ContextFutExt;
use crate::http::server::stream::{jitter, ActiveRequestsGuard};
use crate::http::server::Error;
#[cfg(feature = "runtime")]
use crate::runtime::spawn;
#[cfg(feature = "opentelemetry")]
use crate::telemetry::opentelemetry::OpenTelemetrySpanExt;

pub struct QuicBackend {
	endpoint: quinn::Endpoint,
	builder: Arc<Builder>,
	handler: crate::context::Handler,
	keep_alive_timeout: Option<std::time::Duration>,
}

impl QuicBackend {
	pub fn new(endpoint: quinn::Endpoint, builder: Arc<Builder>, ctx: &crate::context::Context) -> Self {
		Self {
			endpoint,
			builder,
			handler: ctx.new_child().1,
			keep_alive_timeout: None,
		}
	}

	pub fn with_keep_alive_timeout(mut self, timeout: impl Into<Option<std::time::Duration>>) -> Self {
		self.keep_alive_timeout = timeout.into();
		self
	}
}

struct IncomingQuicConnection<'a> {
	remote_addr: std::net::SocketAddr,
	connection: &'a Connecting,
}

impl IncomingConnection for IncomingQuicConnection<'_> {
	fn socket_kind(&self) -> SocketKind {
		SocketKind::Quic
	}

	fn remote_addr(&self) -> std::net::SocketAddr {
		self.remote_addr
	}

	fn local_addr(&self) -> Option<std::net::SocketAddr> {
		None
	}

	fn downcast<T: 'static>(&self) -> Option<&T> {
		if std::any::TypeId::of::<T>() == std::any::TypeId::of::<Connecting>() {
			// Safety: Connection is valid for the lifetime of self and the type is correct.
			Some(unsafe { &*(self.connection as *const Connecting as *const T) })
		} else {
			None
		}
	}
}

impl Backend for QuicBackend {
	async fn serve(self, make_service: impl MakeService) -> Result<crate::context::Handler, crate::http::server::Error> {
		tracing::debug!("listening for incoming connections on {:?}", self.endpoint.local_addr()?);
		loop {
			let ctx = self.handler.context();

			tracing::trace!("waiting for incoming connection");

			let Some(Some(connection)) = self.endpoint.accept().with_context(&ctx).await else {
				break;
			};

			if !connection.remote_address_validated() {
				if let Err(err) = connection.retry() {
					tracing::debug!(error = %err, "failed to retry quic connection");
				}

				continue;
			}

			let connection = match connection.accept() {
				Ok(connection) => connection,
				Err(e) => {
					tracing::debug!(error = %e, "failed to accept quic connection");
					continue;
				}
			};

			let span = tracing::trace_span!("connection", remote_addr = %connection.remote_address());
			let _guard = span.enter();
			tracing::trace!("connection accepted");

			let Some(service) = make_service
				.make_service(&IncomingQuicConnection {
					remote_addr: connection.remote_address(),
					connection: &connection,
				})
				.await
			else {
				tracing::trace!("no service returned for connection, closing");
				continue;
			};

			tracing::trace!("spawning connection handler");

			spawn(
				Connection {
					connection,
					builder: self.builder.clone(),
					service,
					keep_alive_timeout: self.keep_alive_timeout,
					parent_ctx: ctx,
				}
				.serve()
				.in_current_span(),
			);
		}

		Ok(self.handler)
	}

	fn handler(&self) -> &crate::context::Handler {
		&self.handler
	}
}

struct Connection<S: ServiceHandler> {
	connection: Connecting,
	builder: Arc<Builder>,
	service: S,
	keep_alive_timeout: Option<std::time::Duration>,
	parent_ctx: crate::context::Context,
}

impl<S: ServiceHandler> Connection<S> {
	async fn serve(self) {
		tracing::trace!("connection handler started");
		let connection = match self.connection.with_context(&self.parent_ctx).await {
			Some(Ok(connection)) => connection,
			Some(Err(err)) => {
				self.service.on_error(err.into()).await;
				self.service.on_close().await;
				return;
			}
			None => {
				self.service.on_close().await;
				return;
			}
		};

		let mut connection = match self
			.builder
			.build(h3_quinn::Connection::new(connection))
			.with_context(&self.parent_ctx)
			.await
		{
			Some(Ok(connection)) => connection,
			Some(Err(err)) => {
				self.service.on_error(err.into()).await;
				self.service.on_close().await;
				return;
			}
			None => {
				self.service.on_close().await;
				return;
			}
		};

		let (hijack_conn_tx, mut hijack_conn_rx) = tokio::sync::mpsc::channel::<SendQuicConnection>(1);

		self.service.on_ready().await;
		#[cfg(feature = "opentelemetry")]
		tracing::Span::current().make_root();
		tracing::trace!("connection ready");

		let (_, handler) = self.parent_ctx.new_child();

		// This handle is similar to the above however, unlike the above if this handle
		// is cancelled, all futures for this connection are immediately cancelled.
		// When the above is cancelled, the connection is allowed to finish.
		let connection_handle = crate::context::Handler::new();

		let active_requests = Arc::new(std::sync::atomic::AtomicUsize::new(0));

		loop {
			let (request, stream) = tokio::select! {
				request = connection.accept() => {
					match request {
						Ok(Some(request)) => request,
						// The connection was closed.
						Ok(None) => {
							tracing::trace!("connection closed");
							connection_handle.cancel();
							break;
						},
						// An error occurred.
						Err(err) => {
							match err.get_error_level() {
								ErrorLevel::ConnectionError => {
									tracing::debug!(err = %err, "error accepting request");
									self.service.on_error(err.into()).await;
									connection_handle.cancel();
									break;
								}
								ErrorLevel::StreamError => {
									if let Some(Code::H3_NO_ERROR) = err.try_get_code() {
										tracing::trace!("stream closed");
									} else {
										tracing::debug!(err = %err, "stream error");
										self.service.on_error(err.into()).await;
									}
									continue;
								}
							}
						}
					}
				},
				Some(_) = async {
					if let Some(keep_alive_timeout) = self.keep_alive_timeout {
						loop {
							tokio::time::sleep(jitter(keep_alive_timeout)).await;
							if active_requests.load(std::sync::atomic::Ordering::Relaxed) != 0 {
								continue;
							}

							break Some(());
						}
					} else {
						None
					}
				} => {
					tracing::debug!("keep alive timeout");
					break;
				}
				// This happens when the connection has been upgraded to a WebTransport connection.
				Some(send_hijack_conn) = hijack_conn_rx.recv() => {
					tracing::trace!("connection hijacked");
					send_hijack_conn.send(connection).ok();
					self.service.on_hijack().await;
					return;
				},
				_ = self.parent_ctx.done() => break,
			};

			tracing::trace!("new request");
			let active_requests = ActiveRequestsGuard::new(active_requests.clone());

			let service = self.service.clone();
			let stream = QuinnStream::new(stream);

			let mut request = request.map(|()| Body::from_stream(QuinnHttpBodyAdapter::new(stream.clone())));

			let ctx = handler.context();

			request.extensions_mut().insert(QuicConnectionState {
				hijack_conn_tx: hijack_conn_tx.clone(),
				stream: stream.clone(),
			});

			request.extensions_mut().insert(SocketKind::Quic);

			request.extensions_mut().insert(ctx.clone());

			let connection_context = connection_handle.context();

			tokio::spawn(
				async move {
					if let Err(err) = serve_request(&service, request, stream).await {
						service.on_error(err).await;
					}

					drop(active_requests);
					drop(ctx);
				}
				.with_context(connection_context)
				.in_current_span(),
			);
		}

		tracing::trace!("connection closing");

		handler.shutdown().await;

		connection_handle.shutdown().await;

		self.service.on_close().await;

		tracing::trace!("connection closed");
	}
}

async fn serve_request(service: &impl ServiceHandler, request: Request, mut stream: QuinnStream) -> Result<(), Error> {
	let response = service.on_request(request).await.into_response();

	let Some(send) = stream.get_send() else {
		// The service was hijacked.
		tracing::trace!("service hijacked, not sending response");
		return Ok(());
	};

	let (parts, body) = response.into_parts();
	tracing::trace!(?parts, "sending response");
	send.send_response(Response::from_parts(parts, ())).await?;

	let mut body = std::pin::pin!(body);

	tracing::trace!("sending response body");

	loop {
		match poll_fn(|cx| body.as_mut().poll_frame(cx)).await.transpose()? {
			Some(frame) => {
				if frame.is_data() {
					let data = frame.into_data().unwrap();
					tracing::trace!(size = data.len(), "sending data");
					send.send_data(data).await?;
				} else if frame.is_trailers() {
					tracing::trace!("sending trailers");
					send.send_trailers(frame.into_trailers().unwrap()).await?;
					break;
				}
			}
			None => {
				send.finish().await?;
				break;
			}
		}
	}

	tracing::trace!("response body finished");

	Ok(())
}

type SendQuicConnection = tokio::sync::oneshot::Sender<h3::server::Connection<h3_quinn::Connection, Bytes>>;

#[derive(Clone)]
struct QuicConnectionState {
	hijack_conn_tx: tokio::sync::mpsc::Sender<SendQuicConnection>,
	stream: QuinnStream,
}

enum SharedStream {
	Bidi(Option<RequestStream<BidiStream<Bytes>, Bytes>>),
	Recv(Option<RequestStream<RecvStream, Bytes>>),
	Send(Option<RequestStream<SendStream<Bytes>, Bytes>>),
}

impl SharedStream {
	fn take_bidi(&mut self) -> Option<RequestStream<BidiStream<Bytes>, Bytes>> {
		match self {
			SharedStream::Bidi(stream) => stream.take(),
			_ => None,
		}
	}

	fn take_recv(&mut self) -> Option<RequestStream<RecvStream, Bytes>> {
		match self {
			SharedStream::Recv(stream) => stream.take(),
			SharedStream::Bidi(stream) => {
				let (send, recv) = stream.take()?.split();
				*self = SharedStream::Send(Some(send));
				Some(recv)
			}
			_ => None,
		}
	}

	fn take_send(&mut self) -> Option<RequestStream<SendStream<Bytes>, Bytes>> {
		match self {
			SharedStream::Send(stream) => stream.take(),
			SharedStream::Bidi(stream) => {
				let (send, recv) = stream.take()?.split();
				*self = SharedStream::Recv(Some(recv));
				Some(send)
			}
			_ => None,
		}
	}
}

enum QuinnStream {
	Shared(Arc<spin::Mutex<SharedStream>>),
	LocalRecv(RequestStream<RecvStream, Bytes>),
	LocalSend(RequestStream<SendStream<Bytes>, Bytes>),
	None,
}

impl Clone for QuinnStream {
	fn clone(&self) -> Self {
		match self {
			QuinnStream::Shared(stream) => QuinnStream::Shared(stream.clone()),
			_ => QuinnStream::None,
		}
	}
}

impl QuinnStream {
	fn new(stream: RequestStream<BidiStream<Bytes>, Bytes>) -> Self {
		QuinnStream::Shared(Arc::new(spin::Mutex::new(SharedStream::Bidi(Some(stream)))))
	}

	fn take_bidi(&mut self) -> Option<RequestStream<BidiStream<Bytes>, Bytes>> {
		match self {
			QuinnStream::Shared(stream) => {
				let stream = stream.lock().take_bidi()?;
				*self = Self::None;
				Some(stream)
			}
			_ => None,
		}
	}

	fn get_recv(&mut self) -> Option<&mut RequestStream<RecvStream, Bytes>> {
		match self {
			QuinnStream::Shared(stream) => {
				let stream = stream.lock().take_recv()?;
				*self = Self::LocalRecv(stream);
				self.get_recv()
			}
			QuinnStream::LocalRecv(stream) => Some(stream),
			_ => None,
		}
	}

	fn get_send(&mut self) -> Option<&mut RequestStream<SendStream<Bytes>, Bytes>> {
		match self {
			QuinnStream::Shared(stream) => {
				let stream = stream.lock().take_send()?;
				*self = Self::LocalSend(stream);
				self.get_send()
			}
			QuinnStream::LocalSend(stream) => Some(stream),
			_ => None,
		}
	}
}

struct QuinnHttpBodyAdapter {
	stream: QuinnStream,
}

impl QuinnHttpBodyAdapter {
	fn new(stream: QuinnStream) -> Self {
		Self { stream }
	}
}

impl futures::Stream for QuinnHttpBodyAdapter {
	type Item = Result<Bytes, h3::Error>;

	fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		let stream = match self.stream.get_recv() {
			Some(stream) => stream,
			None => return Poll::Ready(None),
		};

		match std::pin::pin!(stream.recv_data()).poll(cx) {
			Poll::Ready(Ok(Some(mut buf))) => Poll::Ready(Some(Ok(buf.copy_to_bytes(buf.remaining())))),
			Poll::Ready(Ok(None)) => Poll::Ready(None),
			Poll::Ready(Err(err)) => Poll::Ready(Some(Err(err))),
			Poll::Pending => Poll::Pending,
		}
	}
}

#[cfg(feature = "http3-webtransport")]
use future::IntoFuture;
#[cfg(feature = "http3-webtransport")]
use h3_webtransport::server::WebTransportSession;

pub struct HijackedQuicConnection {
	pub connection: h3::server::Connection<h3_quinn::Connection, Bytes>,
	pub stream: RequestStream<BidiStream<Bytes>, Bytes>,
	pub request: Request<()>,
}

impl HijackedQuicConnection {
	#[cfg(feature = "http3-webtransport")]
	pub async fn upgrade_webtransport(self) -> Result<WebTransportSession<h3_quinn::Connection, Bytes>, Error> {
		Ok(WebTransportSession::accept(self.request, self.stream, self.connection).await?)
	}
}

pub async fn is_webtransport(request: &Request) -> bool {
	request.method() == http::Method::CONNECT
		&& request.extensions().get::<Protocol>() == Some(&Protocol::WEB_TRANSPORT)
		&& request.extensions().get::<QuicConnectionState>().is_some()
}

pub async fn hijack_quic_connection(request: Request) -> Result<HijackedQuicConnection, Request> {
	let Some(web_transport_state) = request.extensions().get::<QuicConnectionState>() else {
		tracing::debug!("request is not a quic connection");
		return Err(request);
	};

	let Some(stream) = web_transport_state.stream.clone().take_bidi() else {
		tracing::debug!("request body has already been read");
		return Err(request);
	};

	let (send, recv) = tokio::sync::oneshot::channel();
	if web_transport_state.hijack_conn_tx.send(send).await.is_err() {
		tracing::debug!("connection has already been hijacked");
		return Err(request);
	}

	let connection = match recv.await {
		Ok(connection) => connection,
		Err(_) => {
			tracing::debug!("connection was dropped");
			return Err(request);
		}
	};

	let request = request.map(|_| {});

	Ok(HijackedQuicConnection {
		connection,
		stream,
		request,
	})
}
