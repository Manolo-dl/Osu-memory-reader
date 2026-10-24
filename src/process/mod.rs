//! Cross-platform access to another process's memory.
//!
//! osu! stable is a 32-bit process (natively on Windows, under Wine on Linux),
//! so pointers are 4 bytes wide.

use std::fmt;
use std::io;

use crate::pattern::Pattern;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux as sys;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows as sys;

/// Hint shown when the memory scan fails, usually because of missing permissions.
pub const PERMISSION_HINT: &str = sys::PERMISSION_HINT;

#[derive(Debug)]
pub struct ReadError {
    pub address: u64,
    pub size: usize,
    pub os: io::Error,
}

impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "no se pudo leer {:#x} ({} bytes): {}", self.address, self.size, self.os)
    }
}

impl std::error::Error for ReadError {}

pub type Result<T> = std::result::Result<T, ReadError>;

pub struct Region {
    pub address: u64,
    pub size: usize,
}

pub struct Process {
    handle: sys::Handle,
}

impl Process {
    /// Returns the ids of the processes whose executable matches one of `names`.
    pub fn find(names: &[&str]) -> Vec<u32> {
        sys::find(names)
    }

    pub fn open(pid: u32) -> io::Result<Self> {
        Ok(Self { handle: sys::Handle::open(pid)? })
    }

    pub fn is_alive(&self) -> bool {
        self.handle.is_alive()
    }

    pub fn read_into(&self, address: u64, buf: &mut [u8]) -> Result<()> {
        self.handle.read(address, buf).map_err(|os| ReadError { address, size: buf.len(), os })
    }

    fn read_array<const N: usize>(&self, address: u64) -> Result<[u8; N]> {
        let mut buf = [0u8; N];
        self.read_into(address, &mut buf)?;
        Ok(buf)
    }

    pub fn read_i32(&self, address: u64) -> Result<i32> {
        self.read_array(address).map(i32::from_le_bytes)
    }

    pub fn read_u32(&self, address: u64) -> Result<u32> {
        self.read_array(address).map(u32::from_le_bytes)
    }

    pub fn read_f32(&self, address: u64) -> Result<f32> {
        self.read_array(address).map(f32::from_le_bytes)
    }

    /// Reads a 32-bit pointer.
    pub fn read_ptr(&self, address: u64) -> Result<u64> {
        self.read_u32(address).map(u64::from)
    }

    /// `[[address]]`: reads the pointer, then the value it points to.
    pub fn read_ptr_ptr(&self, address: u64) -> Result<u64> {
        self.read_ptr(self.read_ptr(address)?)
    }

    /// Reads a 32-bit .NET `System.String`: length at `+0x4`, UTF-16 data at `+0x8`.
    pub fn read_cs_string(&self, address: u64) -> Result<String> {
        if address == 0 {
            return Ok(String::new());
        }

        let len = self.read_i32(address + 0x4)?;
        if len <= 0 || len >= 4096 {
            return Ok(String::new());
        }

        let mut buf = vec![0u8; len as usize * 2];
        self.read_into(address + 0x8, &mut buf)?;

        let units: Vec<u16> = buf.as_chunks::<2>().0.iter().map(|&c| u16::from_le_bytes(c)).collect();
        Ok(String::from_utf16_lossy(&units))
    }

    /// Reads the pointer at `address`, then the string it points to.
    pub fn read_cs_string_ptr(&self, address: u64) -> Result<String> {
        self.read_cs_string(self.read_ptr(address)?)
    }

    /// Committed memory regions that are readable and writable.
    pub fn regions(&self) -> Vec<Region> {
        self.handle.regions()
    }

    /// Searches for several patterns, reading each region only once.
    /// Returns, for each pattern, the address where its match starts.
    pub fn scan_batch(&self, patterns: &[Pattern]) -> Vec<Option<u64>> {
        let mut results = vec![None; patterns.len()];
        let mut buf = Vec::new();

        for region in self.regions() {
            buf.resize(region.size, 0);
            if self.read_into(region.address, &mut buf).is_err() {
                continue;
            }

            for (pattern, result) in patterns.iter().zip(results.iter_mut()) {
                if result.is_some() {
                    continue;
                }
                if let Some(offset) = pattern.find_in(&buf) {
                    *result = Some(region.address + offset as u64);
                }
            }

            if results.iter().all(Option::is_some) {
                break;
            }
        }

        results
    }
}
