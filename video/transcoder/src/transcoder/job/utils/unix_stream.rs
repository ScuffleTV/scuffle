use std::io;
use std::path::Path;
use std::time::Duration;

use bytes::Bytes;
use bytesio::bytesio::BytesIO;
use bytesio::bytesio_errors::BytesIOError;
use common::prelude::FutureTimeout;
use tokio::net::UnixListener;

pub fn bind_socket(path: &Path, uid: u32, gid: u32) -> anyhow::Result<UnixListener> {
	tracing::debug!(sock_path = %path.display(), "creating socket");
	let socket = match UnixListener::bind(path) {
		Ok(s) => s,
		Err(err) => {
			anyhow::bail!("failed to bind socket: {}", err)
		}
	};

	// Change user and group of the socket.
	if let Err(err) = nix::unistd::chown(
		path.as_os_str(),
		Some(nix::unistd::Uid::from_raw(uid)),
		Some(nix::unistd::Gid::from_raw(gid)),
	) {
		anyhow::bail!("failed to change ownership socket: {}", err)
	}

	Ok(socket)
}

pub fn unix_stream(listener: UnixListener, buffer_size: usize) -> impl futures::Stream<Item = io::Result<Bytes>> + Send {
	async_stream::stream!({
		let (sock, _) = match listener.accept().timeout(Duration::from_secs(4)).await {
			Ok(Ok(connection)) => connection,
			Ok(Err(err)) => {
				yield Err(err);
				return;
			}
			Err(_) => {
				// Timeout
				tracing::debug!("unix stream timeout");
				return;
			}
		};

		tracing::debug!("accepted connection");

		let mut bio = BytesIO::with_capacity(sock, buffer_size);

		loop {
			match bio.read().await {
				Ok(bytes) => {
					yield Ok(bytes.freeze());
				}
				Err(err) => match err {
					BytesIOError::ClientClosed => {
						return;
					}
					_ => {
						yield Err(io::Error::new(
							io::ErrorKind::UnexpectedEof,
							anyhow::anyhow!("failed to read from socket: {}", err),
						));
					}
				},
			}
		}
	})
}
