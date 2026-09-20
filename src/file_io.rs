//! File generation checks and exact-length, backpressured download readers.
//! These checks do not replace authorization or an atomic directory sandbox.
use std::fs::Metadata;
use std::io;
use std::path::Path;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::SystemTime;
use tokio::io::{AsyncRead, AsyncReadExt, ReadBuf};

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub(super) struct FileVersion {
    len: u64,
    modified: Option<SystemTime>,
    created: Option<SystemTime>,
    #[cfg(unix)]
    identity: (u64, u64, i64, i64),
}

impl FileVersion {
    pub(super) fn new(metadata: &Metadata) -> Self {
        #[cfg(unix)]
        use std::os::unix::fs::MetadataExt;
        Self {
            len: metadata.len(),
            modified: metadata.modified().ok(),
            created: metadata.created().ok(),
            #[cfg(unix)]
            identity: (
                metadata.dev(),
                metadata.ino(),
                metadata.ctime(),
                metadata.ctime_nsec(),
            ),
        }
    }

    pub(super) fn check(self, metadata: &Metadata) -> io::Result<()> {
        if metadata.is_file() && self == Self::new(metadata) {
            Ok(())
        } else {
            Err(io::Error::other("file changed before download read"))
        }
    }
}

pub(super) async fn open_checked(path: &Path, expected: &Metadata) -> io::Result<tokio::fs::File> {
    let file = tokio::fs::File::open(path).await?;
    FileVersion::new(expected).check(&file.metadata().await?)?;
    Ok(file)
}

/// A normal `take` bounds growth but silently accepts early EOF. All transports
/// need a body error on truncation, including H3 which otherwise sends FIN.
pub(super) struct ExactReader<R> {
    source: tokio::io::Take<R>,
}

impl<R: AsyncRead> ExactReader<R> {
    pub(super) fn new(source: R, len: u64) -> Self {
        Self {
            source: source.take(len),
        }
    }
}

impl<R: AsyncRead + Unpin> AsyncRead for ExactReader<R> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        if buf.remaining() == 0 || self.source.limit() == 0 {
            return Poll::Ready(Ok(()));
        }
        let before = buf.filled().len();
        match Pin::new(&mut self.source).poll_read(cx, buf) {
            Poll::Ready(Ok(())) if buf.filled().len() == before => {
                Poll::Ready(Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "download source ended before Content-Length",
                )))
            }
            result => result,
        }
    }
}
