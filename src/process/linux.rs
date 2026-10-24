//! Linux backend: `/proc` and `process_vm_readv`.

use std::ffi::c_void;
use std::fs;
use std::io;

use super::Region;

pub const PERMISSION_HINT: &str = "prueba con sudo o setcap cap_sys_ptrace=eip";

#[repr(C)]
struct IoVec {
    base: *mut c_void,
    len: usize,
}

unsafe extern "C" {
    fn process_vm_readv(
        pid: i32,
        local_iov: *const IoVec,
        liovcnt: u64,
        remote_iov: *const IoVec,
        riovcnt: u64,
        flags: u64,
    ) -> isize;
}

/// Finds processes whose `/proc/<pid>/comm` contains one of the names.
pub fn find(names: &[&str]) -> Vec<u32> {
    let Ok(entries) = fs::read_dir("/proc") else {
        return Vec::new();
    };

    let mut pids = Vec::new();
    for entry in entries.flatten() {
        let Some(pid) = entry.file_name().to_str().and_then(|s| s.parse::<u32>().ok()) else {
            continue;
        };
        let Ok(comm) = fs::read_to_string(format!("/proc/{pid}/comm")) else {
            continue;
        };
        if names.iter().any(|name| comm.contains(name)) {
            pids.push(pid);
        }
    }
    pids
}

/// On Linux there is nothing to open; the pid is enough.
pub struct Handle {
    pid: u32,
}

impl Handle {
    pub fn open(pid: u32) -> io::Result<Self> {
        Ok(Self { pid })
    }

    pub fn is_alive(&self) -> bool {
        fs::metadata(format!("/proc/{}", self.pid)).is_ok()
    }

    pub fn read(&self, address: u64, buf: &mut [u8]) -> io::Result<()> {
        let local = IoVec { base: buf.as_mut_ptr().cast(), len: buf.len() };
        let remote = IoVec { base: address as *mut c_void, len: buf.len() };

        let read = unsafe { process_vm_readv(self.pid as i32, &local, 1, &remote, 1, 0) };
        if read < 0 {
            return Err(io::Error::last_os_error());
        }
        if read as usize != buf.len() {
            return Err(io::Error::from(io::ErrorKind::UnexpectedEof));
        }
        Ok(())
    }

    /// Readable and writable regions according to `/proc/<pid>/maps`.
    pub fn regions(&self) -> Vec<Region> {
        let Ok(maps) = fs::read_to_string(format!("/proc/{}/maps", self.pid)) else {
            return Vec::new();
        };

        maps.lines()
            .filter_map(|line| {
                let mut parts = line.split_whitespace();
                let range = parts.next()?;
                let perms = parts.next()?;
                if !perms.starts_with("rw") {
                    return None;
                }

                let (start, end) = range.split_once('-')?;
                let start = u64::from_str_radix(start, 16).ok()?;
                let end = u64::from_str_radix(end, 16).ok()?;
                Some(Region { address: start, size: (end - start) as usize })
            })
            .collect()
    }
}
