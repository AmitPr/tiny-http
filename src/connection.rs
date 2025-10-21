//! Abstractions of Tcp and Unix socket types

#[cfg(unix)]
use std::os::unix::net as unix_net;
use std::{
    net::{Shutdown, SocketAddr, TcpListener, TcpStream, ToSocketAddrs},
    path::PathBuf,
};

/// Unified listener. Either a [`TcpListener`] or [`std::os::unix::net::UnixListener`]
pub enum Listener {
    Tcp(TcpListener),
    #[cfg(unix)]
    Unix(unix_net::UnixListener),
    #[cfg(feature = "vsock")]
    Vsock(vsock::VsockListener),
}
impl Listener {
    pub(crate) fn local_addr(&self) -> std::io::Result<ListenAddr> {
        match self {
            Self::Tcp(l) => l.local_addr().map(ListenAddr::from),
            #[cfg(unix)]
            Self::Unix(l) => l.local_addr().map(ListenAddr::from),
            #[cfg(feature = "vsock")]
            Self::Vsock(l) => l.local_addr().map(ListenAddr::from),
        }
    }

    pub(crate) fn accept(&self) -> std::io::Result<(Connection, Option<SocketAddr>)> {
        match self {
            Self::Tcp(l) => l
                .accept()
                .map(|(conn, addr)| (Connection::from(conn), Some(addr))),
            #[cfg(unix)]
            Self::Unix(l) => l.accept().map(|(conn, _)| (Connection::from(conn), None)),
            #[cfg(feature = "vsock")]
            Self::Vsock(l) => l.accept().map(|(conn, _)| (Connection::from(conn), None)),
        }
    }
}
impl From<TcpListener> for Listener {
    fn from(s: TcpListener) -> Self {
        Self::Tcp(s)
    }
}
#[cfg(unix)]
impl From<unix_net::UnixListener> for Listener {
    fn from(s: unix_net::UnixListener) -> Self {
        Self::Unix(s)
    }
}

#[cfg(feature = "vsock")]
impl From<vsock::VsockListener> for Listener {
    fn from(s: vsock::VsockListener) -> Self {
        Self::Vsock(s)
    }
}

/// Unified connection. Either a [`TcpStream`] or [`std::os::unix::net::UnixStream`].
#[derive(Debug)]
pub(crate) enum Connection {
    Tcp(TcpStream),
    #[cfg(unix)]
    Unix(unix_net::UnixStream),
    #[cfg(feature = "vsock")]
    Vsock(vsock::VsockStream),
}
impl std::io::Read for Connection {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Self::Tcp(s) => s.read(buf),
            #[cfg(unix)]
            Self::Unix(s) => s.read(buf),
            #[cfg(feature = "vsock")]
            Self::Vsock(s) => s.read(buf),
        }
    }
}
impl std::io::Write for Connection {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::Tcp(s) => s.write(buf),
            #[cfg(unix)]
            Self::Unix(s) => s.write(buf),
            #[cfg(feature = "vsock")]
            Self::Vsock(s) => s.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::Tcp(s) => s.flush(),
            #[cfg(unix)]
            Self::Unix(s) => s.flush(),
            #[cfg(feature = "vsock")]
            Self::Vsock(s) => s.flush(),
        }
    }
}
impl Connection {
    /// Gets the peer's address. Some for TCP, None for Unix sockets.
    pub(crate) fn peer_addr(&mut self) -> std::io::Result<Option<SocketAddr>> {
        match self {
            Self::Tcp(s) => s.peer_addr().map(Some),
            #[cfg(unix)]
            Self::Unix(_) => Ok(None),
            #[cfg(feature = "vsock")]
            Self::Vsock(_) => Ok(None),
        }
    }

    pub(crate) fn shutdown(&self, how: Shutdown) -> std::io::Result<()> {
        match self {
            Self::Tcp(s) => s.shutdown(how),
            #[cfg(unix)]
            Self::Unix(s) => s.shutdown(how),
            #[cfg(feature = "vsock")]
            Self::Vsock(s) => s.shutdown(how),
        }
    }

    pub(crate) fn try_clone(&self) -> std::io::Result<Self> {
        match self {
            Self::Tcp(s) => s.try_clone().map(Self::from),
            #[cfg(unix)]
            Self::Unix(s) => s.try_clone().map(Self::from),
            #[cfg(feature = "vsock")]
            Self::Vsock(s) => s.try_clone().map(Self::from),
        }
    }
}
impl From<TcpStream> for Connection {
    fn from(s: TcpStream) -> Self {
        Self::Tcp(s)
    }
}
#[cfg(unix)]
impl From<unix_net::UnixStream> for Connection {
    fn from(s: unix_net::UnixStream) -> Self {
        Self::Unix(s)
    }
}

#[cfg(feature = "vsock")]
impl From<vsock::VsockStream> for Connection {
    fn from(s: vsock::VsockStream) -> Self {
        Self::Vsock(s)
    }
}

#[derive(Debug, Clone)]
pub enum ConfigListenAddr {
    IP(Vec<SocketAddr>),
    #[cfg(unix)]
    // TODO: use SocketAddr when bind_addr is stabilized
    Unix(std::path::PathBuf),
    #[cfg(feature = "vsock")]
    Vsock(u32, u32),
}
impl ConfigListenAddr {
    pub fn from_socket_addrs<A: ToSocketAddrs>(addrs: A) -> std::io::Result<Self> {
        addrs.to_socket_addrs().map(|it| Self::IP(it.collect()))
    }

    #[cfg(unix)]
    pub fn unix_from_path<P: Into<PathBuf>>(path: P) -> Self {
        Self::Unix(path.into())
    }

    #[cfg(feature = "vsock")]
    pub fn vsock_from_cid_port(cid: u32, port: u32) -> Self {
        Self::Vsock(cid, port)
    }

    #[cfg(feature = "vsock")]
    pub fn vsock_from_addr(addr: vsock::VsockAddr) -> Self {
        Self::Vsock(addr.cid(), addr.port())
    }

    pub(crate) fn bind(&self) -> std::io::Result<Listener> {
        match self {
            Self::IP(a) => TcpListener::bind(a.as_slice()).map(Listener::from),
            #[cfg(unix)]
            Self::Unix(a) => unix_net::UnixListener::bind(a).map(Listener::from),
            #[cfg(feature = "vsock")]
            Self::Vsock(cid, port) => {
                vsock::VsockListener::bind_with_cid_port(*cid, *port).map(Listener::from)
            }
        }
    }
}

/// Unified listen socket address. Either a [`SocketAddr`] or [`std::os::unix::net::SocketAddr`].
#[derive(Debug, Clone)]
pub enum ListenAddr {
    IP(SocketAddr),
    #[cfg(unix)]
    Unix(unix_net::SocketAddr),
    #[cfg(feature = "vsock")]
    Vsock(vsock::VsockAddr),
}
impl ListenAddr {
    pub fn to_ip(self) -> Option<SocketAddr> {
        match self {
            Self::IP(s) => Some(s),
            #[cfg(unix)]
            Self::Unix(_) => None,
            #[cfg(feature = "vsock")]
            Self::Vsock(_) => None,
        }
    }

    /// Gets the Unix socket address.
    ///
    /// This is also available on non-Unix platforms, for ease of use, but always returns `None`.
    #[cfg(unix)]
    pub fn to_unix(self) -> Option<unix_net::SocketAddr> {
        match self {
            Self::IP(_) => None,
            Self::Unix(s) => Some(s),
            #[cfg(feature = "vsock")]
            Self::Vsock(_) => None,
        }
    }
    #[cfg(not(unix))]
    pub fn to_unix(self) -> Option<SocketAddr> {
        None
    }

    #[cfg(feature = "vsock")]
    pub fn to_vsock(self) -> Option<vsock::VsockAddr> {
        match self {
            Self::IP(_) => None,
            #[cfg(unix)]
            Self::Unix(_) => None,
            Self::Vsock(s) => Some(s),
        }
    }
}
impl From<SocketAddr> for ListenAddr {
    fn from(s: SocketAddr) -> Self {
        Self::IP(s)
    }
}
#[cfg(unix)]
impl From<unix_net::SocketAddr> for ListenAddr {
    fn from(s: unix_net::SocketAddr) -> Self {
        Self::Unix(s)
    }
}
#[cfg(feature = "vsock")]
impl From<vsock::VsockAddr> for ListenAddr {
    fn from(s: vsock::VsockAddr) -> Self {
        Self::Vsock(s)
    }
}
impl std::fmt::Display for ListenAddr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IP(s) => s.fmt(f),
            #[cfg(unix)]
            Self::Unix(s) => std::fmt::Debug::fmt(s, f),
            #[cfg(feature = "vsock")]
            Self::Vsock(s) => std::fmt::Debug::fmt(s, f),
        }
    }
}
