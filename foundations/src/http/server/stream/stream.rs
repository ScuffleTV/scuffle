use std::convert::Infallible;
use std::pin::{pin, Pin};
use std::sync::Arc;
use std::task::{Context, Poll};

use axum::body::Body;
use axum::extract::Request;
use bytes::{Buf, Bytes};
use futures::{Future, StreamExt};
use h3::server::RequestStream;
use h3_quinn::{BidiStream, RecvStream, SendStream};
use hyper::body::Incoming;
use hyper::rt::Executor;
use hyper_util::rt::TokioIo;
use tower::ServiceExt;

use super::builder::RuntimeExecutor;
use super::Error;

pub enum ServerBackend {
	Quic(QuicBackend),
	Tcp(TcpBackend),
	Tls(TlsBackend),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StreamKind {
	Quic,
	Tcp,
}

impl ServerBackend {
	pub async fn accept(&self) -> Result<IncomingConnection, Error> {
		match self {
			ServerBackend::Quic(backend) => {
				let incoming = backend.endpoint.accept().await.ok_or(Error::QuinnClosed)?;
				let addr = incoming.remote_address();
				Ok(IncomingConnection::Quic {
					incoming,
					addr,
					builder: backend.builder.clone(),
				})
			}
			ServerBackend::Tcp(backend) => {
				let (stream, addr) = backend.listener.accept().await?;
				Ok(IncomingConnection::Tcp {
					stream,
					addr,
					builder: backend.builder.clone(),
				})
			}
			ServerBackend::Tls(backend) => {
				let (stream, addr) = backend.tcp_listener.accept().await?;
				Ok(IncomingConnection::Tls {
					stream,
					addr,
					builder: backend.builder.clone(),
					acceptor: backend.acceptor.clone(),
				})
			}
		}
	}
}

pub enum IncomingConnection {
	Quic {
		incoming: quinn::Connecting,
		addr: std::net::SocketAddr,
		builder: Arc<h3::server::Builder>,
	},
	Tcp {
		stream: tokio::net::TcpStream,
		addr: std::net::SocketAddr,
		builder: Arc<hyper_util::server::conn::auto::Builder<RuntimeExecutor>>,
	},
	Tls {
		stream: tokio::net::TcpStream,
		addr: std::net::SocketAddr,
		builder: Arc<hyper_util::server::conn::auto::Builder<RuntimeExecutor>>,
		acceptor: Arc<tokio_rustls::TlsAcceptor>,
	},
}

impl IncomingConnection {
	pub fn remote_addr(&self) -> std::net::SocketAddr {
		match self {
			IncomingConnection::Quic { addr, .. } => *addr,
			IncomingConnection::Tcp { addr, .. } => *addr,
			IncomingConnection::Tls { addr, .. } => *addr,
		}
	}

	pub fn kind(&self) -> StreamKind {
		match self {
			IncomingConnection::Quic { .. } => StreamKind::Quic,
			IncomingConnection::Tcp { .. } => StreamKind::Tcp,
			IncomingConnection::Tls { .. } => StreamKind::Tcp,
		}
	}

	pub async fn accept(self) -> Result<Connection, Error> {
		match self {
			IncomingConnection::Quic { incoming, builder, addr } => {
				let connection = incoming.await?;
				let connection = builder.build(h3_quinn::Connection::new(connection)).await?;
				Ok(Connection::Quic { connection, addr })
			}
			IncomingConnection::Tcp { stream, addr, builder } => Ok(Connection::Tcp { stream, addr, builder }),
			IncomingConnection::Tls {
				stream,
				addr,
				builder,
				acceptor,
			} => {
				let stream = acceptor.accept(stream).await?;
				Ok(Connection::Tls { stream, addr, builder })
			}
		}
	}
}

pub enum Connection {
	Quic {
		connection: h3::server::Connection<h3_quinn::Connection, Bytes>,
		addr: std::net::SocketAddr,
	},
	Tcp {
		stream: tokio::net::TcpStream,
		addr: std::net::SocketAddr,
		builder: Arc<hyper_util::server::conn::auto::Builder<RuntimeExecutor>>,
	},
	Tls {
		stream: tokio_rustls::server::TlsStream<tokio::net::TcpStream>,
		addr: std::net::SocketAddr,
		builder: Arc<hyper_util::server::conn::auto::Builder<RuntimeExecutor>>,
	},
}

pub struct IncomingStream<'a> {
	stream: &'a Connection,
}

type SendQuicConnection = tokio::sync::oneshot::Sender<h3::server::Connection<h3_quinn::Connection, Bytes>>;

impl IncomingStream<'_> {
	pub fn remote_addr(&self) -> std::net::SocketAddr {
		self.stream.remote_addr()
	}

	pub fn kind(&self) -> StreamKind {
		self.stream.kind()
	}
}

impl std::fmt::Debug for IncomingStream<'_> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("IncomingStream")
			.field("remote_addr", &self.remote_addr())
			.field("kind", &self.kind())
			.finish()
	}
}

impl Connection {
	pub fn remote_addr(&self) -> std::net::SocketAddr {
		match self {
			Connection::Quic { addr, .. } => *addr,
			Connection::Tcp { addr, .. } => *addr,
			Connection::Tls { addr, .. } => *addr,
		}
	}

	pub fn kind(&self) -> StreamKind {
		match self {
			Connection::Quic { .. } => StreamKind::Quic,
			Connection::Tcp { .. } => StreamKind::Tcp,
			Connection::Tls { .. } => StreamKind::Tcp,
		}
	}

	pub(super) async fn serve_connection<M, S>(self, mut layer: M) -> Result<(), Error>
	where
		M: for<'a> tower::Service<IncomingStream<'a>, Response = S, Error = Infallible> + Send + 'static,
		S: tower::Service<axum::extract::Request, Response = axum::response::Response, Error = Infallible>
			+ Clone
			+ 'static
			+ Send,
		S::Future: Send + 'static,
	{
		let service: S = layer
			.call(IncomingStream { stream: &self })
			.await
			.unwrap_or_else(|err| match err {});

		match self {
			Connection::Quic { mut connection, .. } => {
				let (free_conn_tx, mut free_conn_rx) = tokio::sync::mpsc::channel::<SendQuicConnection>(1);
				loop {
					let (request, stream) = tokio::select! {
						request = connection.accept() => {
							request?.ok_or(Error::ConnectionClosed)?
						}
						// This happens when the connection has been upgraded to a WebTransport connection.
						Some(free_conn) = free_conn_rx.recv() => {
							free_conn.send(connection).ok();
							return Ok(());
						}
					};

					let mut service = service.clone();
					let mut stream = QuinnStream::new(stream);
					let webtransport_ext = WebTransportExt {
						free_conn_tx: free_conn_tx.clone(),
						stream: stream.clone(),
					};
					let mut request =
						request.map(|()| axum::body::Body::from_stream(QuinnHttpBodyAdapter { stream: stream.clone() }));
					request.extensions_mut().insert(webtransport_ext);

					RuntimeExecutor.execute(async move {
						let response = service.call(request).await.unwrap_or_else(|err| match err {});

						let mut send_stream = stream.clone();
						let Some(send) = send_stream.get_send() else {
							return;
						};
						
						stream.get_recv();
						drop(stream);

						let (parts, body) = response.into_parts();
						let response = axum::response::Response::from_parts(parts, ());

						if let Err(err) = send.send_response(response).await {
							tracing::error!(%err, "failed to send response");
							return;
						}

						let mut stream = body.into_data_stream();
						while let Some(data) = stream.next().await {
							match data {
								Ok(data) => {
									if let Err(err) = send.send_data(data.clone()).await {
										tracing::error!(%err, "failed to send response body");
										return;
									}

									tracing::info!("response body sent");
								}
								Err(err) => {
									tracing::error!(%err, "failed to read response body");
									return;
								}
							}
						}

						if let Err(err) = send.finish().await {
							tracing::error!(%err, "failed to send response body");
							return;
						}

						tracing::info!("response sent");
					});
				}
			}
			Connection::Tcp { stream, builder, .. } => {
				builder
					.serve_connection_with_upgrades(TokioIo::new(stream), TowerToHyperService { service })
					.await?;
			}
			Connection::Tls { stream, builder, .. } => {
				builder
					.serve_connection_with_upgrades(TokioIo::new(stream), TowerToHyperService { service })
					.await?;
			}
		}

		Ok(())
	}
}

#[derive(Clone)]
struct WebTransportExt {
	free_conn_tx: tokio::sync::mpsc::Sender<SendQuicConnection>,
	stream: QuinnStream,
}

struct QuinnHttpBodyAdapter {
	stream: QuinnStream,
}

impl futures::Stream for QuinnHttpBodyAdapter {
	type Item = Result<Bytes, h3::Error>;

	fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		let stream = match self.stream.get_recv() {
			Some(stream) => stream,
			None => return Poll::Ready(None),
		};

		match pin!(stream.recv_data()).poll(cx) {
			Poll::Ready(Ok(Some(mut buf))) => {
				// The buf here isnt a Bytes but an impl Buf.
				// We need to convert it to a Bytes.
				let buf = buf.copy_to_bytes(buf.remaining());
				Poll::Ready(Some(Ok(buf)))
			}
			Poll::Ready(Ok(None)) => Poll::Ready(None),
			Poll::Ready(Err(err)) => Poll::Ready(Some(Err(err))),
			Poll::Pending => Poll::Pending,
		}
	}
}

#[derive(Debug, Copy, Clone)]
struct TowerToHyperService<S> {
	service: S,
}

impl<S> hyper::service::Service<Request<Incoming>> for TowerToHyperService<S>
where
	S: tower::Service<Request> + Clone,
{
	type Error = S::Error;
	type Future = TowerToHyperServiceFuture<S, Request>;
	type Response = S::Response;

	fn call(&self, req: Request<Incoming>) -> Self::Future {
		let req = req.map(Body::new);
		TowerToHyperServiceFuture {
			future: self.service.clone().oneshot(req),
		}
	}
}

#[pin_project::pin_project]
struct TowerToHyperServiceFuture<S, R>
where
	S: tower::Service<R>,
{
	#[pin]
	future: tower::util::Oneshot<S, R>,
}

impl<S, R> std::future::Future for TowerToHyperServiceFuture<S, R>
where
	S: tower::Service<R>,
{
	type Output = Result<S::Response, S::Error>;

	#[inline]
	fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		self.project().future.poll(cx)
	}
}

pub struct QuicBackend {
	endpoint: quinn::Endpoint,
	builder: Arc<h3::server::Builder>,
}

impl QuicBackend {
	pub fn new(endpoint: quinn::Endpoint, builder: Arc<h3::server::Builder>) -> Self {
		Self { endpoint, builder }
	}
}

pub struct TcpBackend {
	listener: tokio::net::TcpListener,
	builder: Arc<hyper_util::server::conn::auto::Builder<RuntimeExecutor>>,
	rewrite_tls: bool,
}

impl TcpBackend {
	pub fn new(listener: tokio::net::TcpListener, builder: Arc<hyper_util::server::conn::auto::Builder<RuntimeExecutor>>, rewrite_tls: bool) -> Self {
		Self {
			listener,
			builder,
			rewrite_tls,
		}
	}
}

pub struct TlsBackend {
	acceptor: Arc<tokio_rustls::TlsAcceptor>,
	builder: Arc<hyper_util::server::conn::auto::Builder<RuntimeExecutor>>,
	tcp_listener: tokio::net::TcpListener,
}

impl TlsBackend {
	pub fn new(
		tcp_listener: tokio::net::TcpListener,
		acceptor: Arc<tokio_rustls::TlsAcceptor>,
		builder: Arc<hyper_util::server::conn::auto::Builder<RuntimeExecutor>>,
	) -> Self {
		Self {
			tcp_listener,
			acceptor,
			builder,
		}
	}
}
