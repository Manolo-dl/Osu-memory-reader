// SPDX-License-Identifier: LGPL-3.0-only
// Copyright (C) 2026 Manolo-dl
//
// Based on tosu v4.26.3 (https://github.com/tosuapp/tosu):
//   packages/tsprocess/lib/memory/memory_linux.cc
//   packages/tsprocess/src/process.ts
// Copyright (C) 2023-2026 Mikhail Babynichev and contributors.
// Modified in October 2026: ported from C++/TypeScript to Rust; added Wine
// drive translation for paths.

//! Linux backend: `/proc` and `process_vm_readv`.

use std::env;
use std::ffi::c_void;
use std::fs;
use std::io;
use std::path::PathBuf;

use super::Region;

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

    /// Wine starts osu! from its own folder, so the cwd is usually it. Otherwise
    /// falls back to the path saved by osu-winello.
    pub fn game_folder(&self) -> Option<PathBuf> {
        let cwd = fs::read_link(format!("/proc/{}/cwd", self.pid)).ok();
        if let Some(cwd) = &cwd
            && cwd.join("osu!.exe").is_file()
        {
            return Some(cwd.clone());
        }

        let data_home = env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")));
        if let Some(data_home) = data_home
            && let Ok(osupath) = fs::read_to_string(data_home.join("osuconfig/osupath"))
        {
            return Some(PathBuf::from(osupath.trim()));
        }

        cwd
    }

    /// Translates `X:\...` through `<WINEPREFIX>/dosdevices/x:` and turns
    /// backslashes into slashes.
    pub fn host_path(&self, path: &str) -> PathBuf {
        let bytes = path.as_bytes();
        if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
            let drive = format!("{}:", (bytes[0] as char).to_ascii_lowercase());
            let rest = path[2..].trim_start_matches(['\\', '/']).replace('\\', "/");
            let device = self.wine_prefix().join("dosdevices").join(drive);
            let root = fs::canonicalize(&device).unwrap_or(device);
            return root.join(rest);
        }

        PathBuf::from(path.replace('\\', "/"))
    }

    /// `WINEPREFIX` from the game's environment, or Wine's default `~/.wine`.
    fn wine_prefix(&self) -> PathBuf {
        let environ = fs::read(format!("/proc/{}/environ", self.pid)).unwrap_or_default();
        let prefix = environ
            .split(|&b| b == 0)
            .find_map(|var| var.strip_prefix(b"WINEPREFIX="))
            .map(|value| PathBuf::from(String::from_utf8_lossy(value).into_owned()));

        prefix.unwrap_or_else(|| PathBuf::from(env::var_os("HOME").unwrap_or_default()).join(".wine"))
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
