use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use tokio::io::ReadBuf;

pin_project_lite::pin_project! {
    pub struct Wiretap<T> {
        #[pin]
        inner: T,
        read: Vec<u8>,
        write: Vec<u8>,
    }
}

impl<T> Wiretap<T> {
    pub fn new(inner: T) -> Self {
        Self {
            inner,
            read: Vec::new(),
            write: Vec::new(),
        }
    }

    pub fn inner(&self) -> &T {
        &self.inner
    }

    pub fn into_inner(self) -> T {
        self.inner
    }

    pub fn read(&self) -> &[u8] {
        &self.read
    }

    pub fn read_mut(&mut self) -> &mut Vec<u8> {
        &mut self.read
    }

    pub fn write(&self) -> &[u8] {
        &self.write
    }

    pub fn write_mut(&mut self) -> &mut Vec<u8> {
        &mut self.write
    }
}

impl<T> tokio::io::AsyncRead for Wiretap<T>
where
    T: tokio::io::AsyncRead,
{
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.project();

        let filled_before = buf.filled().len();
        let result = futures::ready!(this.inner.poll_read(cx, buf));
        this.read.extend_from_slice(&buf.filled()[filled_before..]);

        Poll::Ready(result)
    }
}

impl<T> tokio::io::AsyncWrite for Wiretap<T>
where
    T: tokio::io::AsyncWrite,
{
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        let this = self.project();

        Poll::Ready(match futures::ready!(this.inner.poll_write(cx, buf)) {
            Ok(n) => {
                this.write.extend_from_slice(&buf[..n]);
                Ok(n)
            }
            Err(err) => Err(err),
        })
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), std::io::Error>> {
        let this = self.project();
        this.inner.poll_flush(cx)
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        let this = self.project();
        this.inner.poll_shutdown(cx)
    }
}
