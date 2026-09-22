//! One process per data directory. Launching again reveals the existing window.
use anyhow::{Context, Result};
use std::{
    fs::{self, File},
    os::unix::net::UnixDatagram,
    path::Path,
};

pub struct Instance {
    _lock: File,
    socket: UnixDatagram,
}

impl Instance {
    pub fn claim(directory: &Path) -> Result<Option<Self>> {
        fs::create_dir_all(directory)?;
        let lock = File::options()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(directory.join("catdo.lock"))?;
        let socket_path = directory.join("catdo.sock");
        match lock.try_lock() {
            Ok(()) => (),
            Err(std::fs::TryLockError::WouldBlock) => {
                UnixDatagram::unbound()?
                    .send_to(b"show", socket_path)
                    .context("CatDo is already starting. Try opening it again in a moment")?;
                return Ok(None);
            }
            Err(error) => return Err(error.into()),
        }
        if socket_path.exists() {
            fs::remove_file(&socket_path)?;
        }
        let socket = UnixDatagram::bind(socket_path)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn subsequent_launch_reveals_first_and_lock_is_released() {
        let directory = tempfile::tempdir().unwrap();
        let first = Instance::claim(directory.path()).unwrap().unwrap();
        assert!(Instance::claim(directory.path()).unwrap().is_none());
        assert!(first.take_show_request());
        assert!(!first.take_show_request());
        drop(first);
        assert!(Instance::claim(directory.path()).unwrap().is_some());
    }
}
