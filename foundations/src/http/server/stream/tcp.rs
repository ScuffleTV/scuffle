use std::convert::Infallible;
use std::sync::Arc;

use axum::body::Body;
use axum::extract::Request;
use axum::response::IntoResponse;
use hyper::body::Incoming;
use hyper::service::service_fn;
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto::Builder;
use tokio::net::{TcpListener, TcpStream};
#[cfg(not(feature = "runtime"))]
use tokio::spawn;
use tracing::Instrument;

use super::{Backend, IncomingConnection, MakeService, ServiceHandler, SocketKind};
use crate::context::ContextFutExt;
use crate::http::server::stream::{jitter, ActiveRequestsGuard};
#[cfg(feature = "runtime")]
use crate::runtime::spawn;
#[cfg(feature = "opentelemetry")]
use crate::telemetry::opentelemetry::OpenTelemetrySpanExt;

pub struct TcpBackend {
	listener: TcpListener,
	builder: Arc<Builder<TokioExecutor>>,
	handler: crate::context::Handler,
	keep_alive_timeout: Option<std::time::Duration>,
}

impl TcpBackend {
	pub fn new(listener: TcpListener, builder: Arc<Builder<TokioExecutor>>, ctx: &crate::context::Context) -> Self {
		Self {
			listener,
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

struct IncomingTcpConnection<'a> {
	remote_addr: std::net::SocketAddr,
	local_addr: std::net::SocketAddr,
	connection: &'a TcpStream,
}

impl<'a> IncomingConnection for IncomingTcpConnection<'a> {
	fn socket_kind(&self) -> SocketKind {
		SocketKind::Tcp
	}

	fn remote_addr(&self) -> std::net::SocketAddr {
		self.remote_addr
	}

	fn local_addr(&self) -> Option<std::net::SocketAddr> {
		Some(self.local_addr)
	}

	fn downcast<T: 'static>(&self) -> Option<&T> {
		if std::any::TypeId::of::<T>() == std::any::TypeId::of::<TcpStream>() {
			// Safety: We know that the type is TcpStream because we checked the type id.
			// We also know that the reference is valid because it is a reference to a field
			// of self.
			Some(unsafe { &*(self.connection as *const TcpStream as *const T) })
		} else {
			None
		}
	}
}

impl Backend for TcpBackend {
	async fn serve(self, make_service: impl MakeService) -> Result<crate::context::Handler, crate::http::server::Error> {
		tracing::debug!("listening for incoming connections on {:?}", self.listener.local_addr()?);
		loop {
			let ctx = self.handler.context();

			tracing::trace!("waiting for incoming connection");

			let Some((connection, addr)) = self.listener.accept().with_context(&ctx).await.transpose()? else {
				break;
			};

			let span = tracing::trace_span!("connection", remote_addr = %addr);

			let _guard = span.enter();

			tracing::trace!("accepted connection");

			let Some(service) = make_service
				.make_service(&IncomingTcpConnection {
					remote_addr: addr,
					local_addr: self.listener.local_addr()?,
					connection: &connection,
				})
				.await
			else {
				tracing::trace!("no service returned for connection, closing");
				continue;
			};

			spawn(
				Connection {
					connection,
					builder: self.builder.clone(),
					service,
					parent_ctx: ctx,
					peer_addr: addr,
					keep_alive_timeout: self.keep_alive_timeout,
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
	connection: TcpStream,
	builder: Arc<Builder<TokioExecutor>>,
	service: S,
	peer_addr: std::net::SocketAddr,
	parent_ctx: crate::context::Context,
	keep_alive_timeout: Option<std::time::Duration>,
}

impl<S: ServiceHandler> Connection<S> {
	async fn serve(self) {
		self.service.on_ready().await;
		#[cfg(feature = "opentelemetry")]
		tracing::Span::current().make_root();
		tracing::trace!("connection ready");

		let (_, handle) = self.parent_ctx.new_child();

		let make_ctx = {
			let handle = handle.clone();
			Arc::new(move || handle.context())
		};

		let active_requests = Arc::new(std::sync::atomic::AtomicUsize::new(0));

		let ip_addr = self.peer_addr.ip();

		let service_fn = {
			let service = self.service.clone();
			let span = tracing::Span::current();
			let active_requests = active_requests.clone();

			service_fn(move |mut req: Request<Incoming>| {
				let service = service.clone();
				let make_ctx = make_ctx.clone();
				let guard = ActiveRequestsGuard::new(active_requests.clone());
				async move {
					let ctx = make_ctx();
					req.extensions_mut().insert(ctx.clone());
					req.extensions_mut().insert(SocketKind::Tcp);
					req.extensions_mut().insert(ip_addr);
					let resp = service.on_request(req.map(Body::new)).await.into_response();
					drop(ctx);
					drop(guard);
					Ok::<_, Infallible>(resp)
				}
				.instrument(span.clone())
			})
		};

		let r = tokio::select! {
			r = self.builder.serve_connection_with_upgrades(TokioIo::new(self.connection), service_fn) => r,
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
				Ok(())
			}
			_ = async {
				self.parent_ctx.done().await;
				handle.shutdown().await;
			} => {
				Ok(())
			}
		};

		if let Err(err) = r {
			self.service.on_error(err.into()).await;
		}

		self.service.on_close().await;
		tracing::trace!("connection closed");
	}
}
