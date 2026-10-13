use std::fs::File;
use std::io;
use std::os::fd::{AsFd, BorrowedFd, OwnedFd};
use std::os::unix::fs::FileTypeExt;

use rustix::fs::{OFlags, fcntl_getfl, fcntl_setfl};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::UnixStream;
use tokio::net::unix::pipe;

pub(crate) struct Stdio {
    pub(crate) reader: Box<dyn AsyncRead + Unpin>,
    pub(crate) writer: Box<dyn AsyncWrite + Unpin>,
    pub(crate) restored_flags: Option<SavedFlags>,
}

/// Original standard input and output descriptor flags, restored on drop.
pub(crate) struct SavedFlags(Vec<(OwnedFd, OFlags)>);

enum Kind {
    Pipe,
    Socket,
    Other,
}

impl Stdio {
    pub(crate) fn open() -> Self {
        let mut restored_flags = SavedFlags(Vec::new());
        let reader = match Self::on_reactor(io::stdin().as_fd(), &mut restored_flags) {
            Some((fd, Kind::Pipe)) => pipe::Receiver::from_owned_fd(fd)
                .map(|reader| Box::new(reader) as Box<dyn AsyncRead + Unpin>)
                .ok(),
            Some((fd, Kind::Socket)) => Self::socket(fd)
                .map(|reader| Box::new(reader) as Box<dyn AsyncRead + Unpin>)
                .ok(),
            _ => None,
        }
        .unwrap_or_else(|| Box::new(tokio::io::stdin()));
        let writer = match Self::on_reactor(io::stdout().as_fd(), &mut restored_flags) {
            Some((fd, Kind::Pipe)) => pipe::Sender::from_owned_fd(fd)
                .map(|writer| Box::new(writer) as Box<dyn AsyncWrite + Unpin>)
                .ok(),
            Some((fd, Kind::Socket)) => Self::socket(fd)
                .map(|writer| Box::new(writer) as Box<dyn AsyncWrite + Unpin>)
                .ok(),
            _ => None,
        }
        .unwrap_or_else(|| Box::new(tokio::io::stdout()));

        Self {
            reader,
            writer,
            restored_flags: (!restored_flags.0.is_empty()).then_some(restored_flags),
        }
    }

    /// Clones pipe or socket `fd` for reactor I/O and saves its original flags for restoration. Returns `None` for other
    /// file types or errors.
    fn on_reactor(fd: BorrowedFd<'_>, restored_flags: &mut SavedFlags) -> Option<(OwnedFd, Kind)> {
        let file = File::from(fd.try_clone_to_owned().ok()?);
        let file_type = file.metadata().ok()?.file_type();
        let kind = if file_type.is_fifo() {
            Kind::Pipe
        } else if file_type.is_socket() {
            Kind::Socket
        } else {
            Kind::Other
        };

        if matches!(kind, Kind::Other) {
            return None;
        }

        let flags = fcntl_getfl(&file).ok()?;

        restored_flags
            .0
            .push((fd.try_clone_to_owned().ok()?, flags));

        Some((OwnedFd::from(file), kind))
    }

    fn socket(fd: OwnedFd) -> io::Result<UnixStream> {
        let socket = std::os::unix::net::UnixStream::from(fd);

        socket.set_nonblocking(true)?;

        UnixStream::from_std(socket)
    }
}

impl Drop for SavedFlags {
    fn drop(&mut self) {
        for (fd, flags) in &self.0 {
            let _ = fcntl_setfl(fd, *flags);
        }
    }
}
