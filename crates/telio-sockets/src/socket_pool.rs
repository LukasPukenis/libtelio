use std::{
    io,
    net::SocketAddr,
    ops::{Deref, DerefMut},
    pin::Pin,
    sync::Arc,
    task::Poll,
};

use socket2::{Domain, Protocol, Socket, Type};

use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::{TcpSocket, TcpStream, ToSocketAddrs, UdpSocket},
};

#[cfg(unix)]
use boringtun::device::MakeExternalBoringtun;

#[cfg(windows)]
use std::os::windows::io::{FromRawSocket, IntoRawSocket};

#[cfg(not(windows))]
use std::os::unix::io::{FromRawFd, IntoRawFd};

#[cfg(unix)]
use crate::native::NativeSocket;

use crate::{native::AsNativeSocket, NativeProtector, Protector, TcpParams, UdpParams};

pub struct External<T: AsNativeSocket> {
    /// Optional so TcpSocket could be transformed into TcpStream
    inner: Option<(T, ArcProtector)>,
}

#[derive(Clone)]
pub struct SocketPool {
    protect: ArcProtector,
}

type ArcProtector = Arc<dyn Protector>;

impl External<TcpSocket> {
    pub async fn connect(mut self, addr: SocketAddr) -> io::Result<External<TcpStream>> {
        if let Some((sock, p)) = self.inner.take() {
            let native = sock.as_native_socket();
            match sock.connect(addr).await {
                Ok(sock) => Ok(External {
                    inner: Some((sock, p)),
                }),
                Err(e) => {
                    p.clean(native);
                    Err(e)
                }
            }
        } else {
            panic!("Attempt to use dropped value!");
        }
    }

    pub async fn connect_timeout(
        mut self,
        addr: SocketAddr,
        timeout: std::time::Duration,
    ) -> io::Result<External<TcpStream>> {
        // This function should be called from async runtime, because of `TcpStream::from_std`, that's why - `pub fn async`
        if let Some((sock, p)) = self.inner.take() {
            let native = sock.as_native_socket();

            // TODO maybe we should drop the attachment to tokio::TcpSocket type, and stick to more `flexible` socket2::Socket ..?
            #[cfg(not(windows))]
            let socket2 = unsafe { Socket::from_raw_fd(sock.into_raw_fd()) };

            #[cfg(windows)]
            let socket2 = unsafe { Socket::from_raw_socket(sock.into_raw_socket()) };

            match socket2.connect_timeout(&addr.into(), timeout) {
                Ok(_) => {
                    // Socket comes out in blocking mode from `connect_timeout` call ...
                    socket2.set_nonblocking(true)?;
                    let inner_stream = TcpStream::from_std(std::net::TcpStream::from(socket2))?;

                    Ok(External {
                        inner: Some((inner_stream, p)),
                    })
                }
                Err(e) => {
                    p.clean(native);
                    Err(e)
                }
            }
        } else {
            panic!("Attempt to use dropped value!");
        }
    }
}

impl<T: AsNativeSocket> Deref for External<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // Value is constructed with Some, only None case is Extern<TcpSocket>.connect
        &self.inner.as_ref().expect("Used after drop!").0
    }
}

impl<T: AsNativeSocket> DerefMut for External<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // Value is constructed with Some, only None case is Extern<TcpSocket>.connect
        &mut self.inner.as_mut().expect("Used after drop!").0
    }
}

impl<T> AsyncRead for External<T>
where
    T: AsNativeSocket + AsyncRead + Unpin,
{
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        if let Some((inner, _)) = self.inner.as_mut() {
            Pin::new(inner).poll_read(cx, buf)
        } else {
            Poll::Pending
        }
    }
}

impl<T> AsyncWrite for External<T>
where
    T: AsNativeSocket + AsyncWrite + Unpin,
{
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        if let Some((inner, _)) = self.inner.as_mut() {
            Pin::new(inner).poll_write(cx, buf)
        } else {
            Poll::Pending
        }
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), io::Error>> {
        if let Some((inner, _)) = self.inner.as_mut() {
            Pin::new(inner).poll_flush(cx)
        } else {
            Poll::Pending
        }
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), io::Error>> {
        if let Some((inner, _)) = self.inner.as_mut() {
            Pin::new(inner).poll_shutdown(cx)
        } else {
            Poll::Pending
        }
    }
}

impl<T: AsNativeSocket> Drop for External<T> {
    fn drop(&mut self) {
        if let Some((sock, p)) = self.inner.take() {
            let native = sock.as_native_socket();
            p.clean(native);
        }
    }
}

impl SocketPool {
    pub fn new<T: Protector + 'static>(protect: T) -> Self {
        Self {
            protect: Arc::new(protect),
        }
    }

    #[cfg(target_os = "linux")]
    pub fn set_fwmark(&self, fwmark: u32) {
        self.protect.set_fwmark(fwmark);
    }

    #[cfg(any(target_os = "macos", target_os = "ios", windows))]
    pub fn set_tunnel_interface(&self, interface: u64) {
        self.protect.set_tunnel_interface(interface);
    }

    pub fn new_external_tcp_v4(
        &self,
        params: Option<TcpParams>,
        #[cfg(target_os = "macos")] force_protect: bool,
    ) -> io::Result<External<TcpSocket>> {
        let ty = Type::STREAM;

        #[cfg(any(target_os = "android", target_os = "linux"))]
        let ty = ty.nonblocking();

        let socket2_socket = Socket::new(Domain::IPV4, ty, Some(Protocol::TCP))?;

        #[cfg(not(any(target_os = "android", target_os = "linux")))]
        socket2_socket.set_nonblocking(true)?;

        if let Some(params) = params {
            params.apply(&socket2_socket);
        }

        self.new_external(
            TcpSocket::from_std_stream(socket2_socket.into()),
            #[cfg(target_os = "macos")]
            force_protect,
        )
    }

    pub async fn new_udp<A: ToSocketAddrs>(
        addr: A,
        params: Option<UdpParams>,
    ) -> io::Result<UdpSocket> {
        let s = UdpSocket::bind(addr).await?;
        let s = Socket::from(s.into_std()?);

        if let Some(params) = params {
            params.apply(&s);
        }

        UdpSocket::from_std(s.into())
    }

    pub async fn new_internal_udp<A: ToSocketAddrs>(
        &self,
        addr: A,
        params: Option<UdpParams>,
    ) -> io::Result<UdpSocket> {
        let sock = Self::new_udp(addr, params).await?;

        #[cfg(any(target_os = "macos", target_os = "ios"))]
        self.protect.make_internal(sock.as_native_socket())?;

        Ok(sock)
    }

    pub async fn new_external_udp<A: ToSocketAddrs>(
        &self,
        addr: A,
        params: Option<UdpParams>,
    ) -> io::Result<External<UdpSocket>> {
        self.new_external(
            Self::new_udp(addr, params).await?,
            #[cfg(target_os = "macos")]
            true,
        )
    }

    /// wraps protect() on android, fmark on linux and interface binding for others
    pub fn make_external<T: AsNativeSocket>(&self, socket: T) {
        let _ = self.protect.make_external(socket.as_native_socket());
    }

    fn new_external<T: AsNativeSocket>(
        &self,
        socket: T,
        #[cfg(target_os = "macos")] force_protect: bool,
    ) -> io::Result<External<T>> {
        // TODO: remove this check once mac integration tests support our binding mechanism
        #[cfg(target_os = "macos")]
        if force_protect {
            self.protect.make_external(socket.as_native_socket())?;
        }
        #[cfg(not(target_os = "macos"))]
        self.protect.make_external(socket.as_native_socket())?;

        Ok(External {
            inner: Some((socket, self.protect.clone())),
        })
    }
}

impl Default for SocketPool {
    fn default() -> Self {
        Self::new(NativeProtector::new().expect("Native protect"))
    }
}

#[cfg(unix)]
impl MakeExternalBoringtun for SocketPool {
    fn make_external(&self, socket: NativeSocket) {
        let _ = self.protect.make_external(socket);
    }
}

#[cfg(test)]
mod tests {
    use std::{net::Ipv4Addr, sync::Mutex};

    use mockall::mock;

    use crate::{native::NativeSocket, Protect};

    use super::*;

    mock! {
        Protector {}
        impl Protector for Protector {
            fn make_external(&self, socket: NativeSocket) -> io::Result<()>;
            fn clean(&self, socket: NativeSocket);
            #[cfg(target_os = "linux")]
            fn set_fwmark(&self, fwmark: u32);
            #[cfg(any(target_os = "macos", windows))]
            fn set_tunnel_interface(&self, interface: u64);
        }
    }

    #[tokio::test]
    async fn create_and_clean_sockets() {
        let mut protect = MockProtector::default();

        let socks = Arc::new(Mutex::new(Vec::new()));

        protect
            .expect_make_external()
            .returning({
                let socks = socks.clone();
                move |s| {
                    socks.lock().unwrap().push(s);
                    Ok(())
                }
            })
            .times(2);

        protect.expect_clean().return_const(()).times(2);

        let pool = SocketPool::new(protect);
        {
            let tcp = pool
                .new_external_tcp_v4(
                    None,
                    #[cfg(target_os = "macos")]
                    true,
                )
                .expect("tcp");
            let udp = pool
                .new_external_udp(SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0)), None)
                .await
                .expect("udp");

            assert_eq!(
                socks.lock().unwrap().clone(),
                vec![tcp.as_native_socket(), udp.as_native_socket()]
            );
        }
    }

    #[tokio::test]
    async fn create_socket_with_protect_fn() {
        let socks = Arc::new(Mutex::new(Vec::new()));
        let protect: Protect = {
            let socks = socks.clone();
            Arc::new(move |s| {
                socks.lock().unwrap().push(s);
            })
        };
        let pool = SocketPool::new(protect);
        let tcp = pool
            .new_external_tcp_v4(
                None,
                #[cfg(target_os = "macos")]
                true,
            )
            .expect("tcp");

        assert_eq!(socks.lock().unwrap().clone(), vec![tcp.as_native_socket()]);
    }
}
