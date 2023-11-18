use async_trait::async_trait;
use tokio::io::AsyncReadExt as _;

#[async_trait]
pub trait AsyncReadExt: tokio::io::AsyncRead + Unpin {
	async fn read_all(&mut self) -> std::io::Result<Vec<u8>> {
		let mut buf = Vec::new();
		self.read_to_end(&mut buf).await?;
		Ok(buf)
	}
}

impl<T: tokio::io::AsyncRead + Unpin> AsyncReadExt for T {}
