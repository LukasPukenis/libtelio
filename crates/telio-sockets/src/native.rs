#[cfg(unix)]
use std::os::unix::io::{AsRawFd, RawFd};
#[cfg(windows)]
use std::os::windows::io::{AsRawSocket, RawSocket};

#[cfg(unix)]
pub type NativeSocket = RawFd;

#[cfg(windows)]
pub type NativeSocket = RawSocket;

pub trait AsNativeSocket {
    fn as_native_socket(&self) -> NativeSocket;
}

#[cfg(unix)]
impl<T> AsNativeSocket for T
where
    T: AsRawFd,
{
    fn as_native_socket(&self) -> NativeSocket {
        self.as_raw_fd()
    }
}

#[cfg(windows)]
impl<T> AsNativeSocket for T
where
    T: AsRawSocket,
{
    fn as_native_socket(&self) -> NativeSocket {
        self.as_raw_socket()
    }
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
pub fn interface_index_from_name(name: &str) -> std::io::Result<u64> {
    let index = unsafe { libc::if_nametoindex(name.as_ptr() as *const i8) };
    if index == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(index as u64)
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
pub fn interface_index_from_tun(tun_fd: RawFd) -> std::io::Result<u64> {
    let index = unsafe {
        let mut name = [0 as libc::c_char; libc::IFNAMSIZ + 1];
        let mut len = libc::IFNAMSIZ as libc::socklen_t;
        if libc::getsockopt(
            tun_fd,
            libc::SYSPROTO_CONTROL,
            libc::UTUN_OPT_IFNAME,
            &mut name as *mut libc::c_char as *mut libc::c_void,
            &mut len as *mut libc::socklen_t,
        ) != 0
        {
            return Err(std::io::Error::last_os_error());
        }
        libc::if_nametoindex(name.as_ptr()) as i32
    };
    if index == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(index as u64)
}
