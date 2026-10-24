//! Windows backend: Toolhelp snapshots, `ReadProcessMemory` and `VirtualQueryEx`.

use std::ffi::c_void;
use std::io;
use std::mem::{size_of, zeroed};
use std::path::PathBuf;

use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE, STILL_ACTIVE};
use windows_sys::Win32::System::Diagnostics::Debug::ReadProcessMemory;
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::System::Memory::{
    MEM_COMMIT, MEMORY_BASIC_INFORMATION, PAGE_EXECUTE_READWRITE, PAGE_GUARD, PAGE_READWRITE, VirtualQueryEx,
};
use windows_sys::Win32::System::Threading::{
    GetExitCodeProcess, OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ,
    QueryFullProcessImageNameW,
};

/// Finds processes whose executable name matches one of the names (case-insensitive).
pub fn find(names: &[&str]) -> Vec<u32> {
    let mut pids = Vec::new();

    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return pids;
        }

        let mut entry: PROCESSENTRY32W = zeroed();
        entry.dwSize = size_of::<PROCESSENTRY32W>() as u32;

        if Process32FirstW(snapshot, &mut entry) != 0 {
            loop {
                let len = entry.szExeFile.iter().position(|&c| c == 0).unwrap_or(entry.szExeFile.len());
                let exe = String::from_utf16_lossy(&entry.szExeFile[..len]);
                if names.iter().any(|name| exe.eq_ignore_ascii_case(name)) {
                    pids.push(entry.th32ProcessID);
                }

                if Process32NextW(snapshot, &mut entry) == 0 {
                    break;
                }
            }
        }

        CloseHandle(snapshot);
    }

    pids
}

pub struct Handle(HANDLE);

impl Handle {
    pub fn open(pid: u32) -> io::Result<Self> {
        let handle = unsafe { OpenProcess(PROCESS_VM_READ | PROCESS_QUERY_INFORMATION, 0, pid) };
        if handle.is_null() {
            return Err(io::Error::last_os_error());
        }
        Ok(Self(handle))
    }

    pub fn is_alive(&self) -> bool {
        let mut code = 0u32;
        unsafe { GetExitCodeProcess(self.0, &mut code) != 0 && code == STILL_ACTIVE as u32 }
    }

    /// Parent folder of the process executable.
    pub fn game_folder(&self) -> Option<PathBuf> {
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let ok = unsafe { QueryFullProcessImageNameW(self.0, PROCESS_NAME_WIN32, buf.as_mut_ptr(), &mut len) };
        if ok == 0 {
            return None;
        }

        let exe = PathBuf::from(String::from_utf16_lossy(&buf[..len as usize]));
        exe.parent().map(PathBuf::from)
    }

    /// Paths in memory are already native paths.
    pub fn host_path(&self, path: &str) -> PathBuf {
        PathBuf::from(path)
    }

    pub fn read(&self, address: u64, buf: &mut [u8]) -> io::Result<()> {
        let mut read = 0usize;
        let ok = unsafe {
            ReadProcessMemory(
                self.0,
                address as *const c_void,
                buf.as_mut_ptr().cast(),
                buf.len(),
                &mut read,
            )
        };
        if ok == 0 {
            return Err(io::Error::last_os_error());
        }
        if read != buf.len() {
            return Err(io::Error::from(io::ErrorKind::UnexpectedEof));
        }
        Ok(())
    }

    /// Committed regions that are readable and writable, walked with `VirtualQueryEx`.
    pub fn regions(&self) -> Vec<super::Region> {
        let mut regions = Vec::new();
        let mut address = 0usize;

        loop {
            let mut info: MEMORY_BASIC_INFORMATION = unsafe { zeroed() };
            let written = unsafe {
                VirtualQueryEx(
                    self.0,
                    address as *const c_void,
                    &mut info,
                    size_of::<MEMORY_BASIC_INFORMATION>(),
                )
            };
            if written == 0 {
                break;
            }

            let writable = info.Protect & (PAGE_READWRITE | PAGE_EXECUTE_READWRITE) != 0;
            let guarded = info.Protect & PAGE_GUARD != 0;
            if info.State == MEM_COMMIT && writable && !guarded {
                regions.push(super::Region { address: info.BaseAddress as u64, size: info.RegionSize });
            }

            match (info.BaseAddress as usize).checked_add(info.RegionSize) {
                Some(next) if next > address => address = next,
                _ => break,
            }
        }

        regions
    }
}

// SAFETY: a process handle is just a kernel object reference; the Win32 calls
// used here are safe to make from any thread.
unsafe impl Send for Handle {}
unsafe impl Sync for Handle {}

impl Drop for Handle {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.0) };
    }
}
