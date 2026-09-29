//! One process per data directory. Launching again reveals the existing window.
use anyhow::{Context, Result};
#[cfg(windows)]
use std::net::{Ipv4Addr, UdpSocket};
#[cfg(unix)]
use std::os::unix::net::UnixDatagram;
use std::{
    fs::{self, File},
    path::Path,
    time::{Duration, Instant},
};

pub struct Instance {
    _lock: File,
    #[cfg(unix)]
    socket: UnixDatagram,
    #[cfg(windows)]
    socket: UdpSocket,
}

impl Instance {
    /// With `wait`, an updated CatDo waits for its predecessor to exit rather
    /// than asking it to show its window.
    pub fn claim(directory: &Path, wait: bool) -> Result<Option<Self>> {
        fs::create_dir_all(directory)?;
        let lock = File::options()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(directory.join("catdo.lock"))?;
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            match lock.try_lock() {
                Ok(()) => break,
                Err(std::fs::TryLockError::WouldBlock) if wait && Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(100));
                }
                Err(std::fs::TryLockError::WouldBlock) => {
                    request_show(directory)
                        .context("CatDo is already starting. Try opening it again in a moment")?;
                    return Ok(None);
                }
                Err(error) => return Err(error.into()),
            }
        }
        let socket = listen(directory)?;
        socket.set_nonblocking(true)?;
        Ok(Some(Self {
            _lock: lock,
            socket,
        }))
    }

    pub fn take_show_request(&self) -> bool {
        let mut buffer = [0; 16];
        matches!(self.socket.recv(&mut buffer), Ok(4)) && &buffer[..4] == b"show"
    }
}

#[cfg(unix)]
fn listen(directory: &Path) -> Result<UnixDatagram> {
    let socket_path = directory.join("catdo.sock");
    if socket_path.exists() {
        fs::remove_file(&socket_path)?;
    }
    Ok(UnixDatagram::bind(socket_path)?)
}

#[cfg(unix)]
fn request_show(directory: &Path) -> Result<()> {
    UnixDatagram::unbound()?.send_to(b"show", directory.join("catdo.sock"))?;
    Ok(())
}

// Windows has no Unix datagram sockets in std: the running instance listens on
// a loopback port recorded beside its lock.
#[cfg(windows)]
fn listen(directory: &Path) -> Result<UdpSocket> {
    let socket = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0))?;
    fs::write(
        directory.join("catdo.port"),
        socket.local_addr()?.port().to_string(),
    )?;
    Ok(socket)
}

#[cfg(windows)]
fn request_show(directory: &Path) -> Result<()> {
    let port: u16 = fs::read_to_string(directory.join("catdo.port"))?
        .trim()
        .parse()?;
    // Let the running instance bring its window to the front.
    unsafe {
        windows_sys::Win32::UI::WindowsAndMessaging::AllowSetForegroundWindow(
            windows_sys::Win32::UI::WindowsAndMessaging::ASFW_ANY,
        );
    }
    UdpSocket::bind((Ipv4Addr::LOCALHOST, 0))?.send_to(b"show", (Ipv4Addr::LOCALHOST, port))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn subsequent_launch_reveals_first_and_lock_is_released() {
        let directory = tempfile::tempdir().unwrap();
        let first = Instance::claim(directory.path(), false).unwrap().unwrap();
        assert!(Instance::claim(directory.path(), false).unwrap().is_none());
        assert!(first.take_show_request());
        assert!(!first.take_show_request());
        drop(first);
        assert!(Instance::claim(directory.path(), false).unwrap().is_some());
    }

    #[test]
    fn restarted_launch_waits_for_previous_instance_to_exit() {
        let directory = tempfile::tempdir().unwrap();
        let first = Instance::claim(directory.path(), false).unwrap().unwrap();
        let exit = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(300));
            drop(first);
        });
        assert!(Instance::claim(directory.path(), true).unwrap().is_some());
        exit.join().unwrap();
    }
}
