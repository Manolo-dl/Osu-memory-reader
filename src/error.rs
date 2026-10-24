// SPDX-License-Identifier: LGPL-3.0-only
// Copyright (C) 2026 Manolo-dl

use std::fmt;
use std::io;
use std::path::PathBuf;

/// Errors returned by [`OsuReader`](crate::OsuReader).
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// No `osu!.exe` process is running.
    ProcessNotFound,

    /// The process exists but could not be opened for reading.
    ///
    /// On Windows this usually means osu! runs as administrator and this
    /// program doesn't.
    OpenProcess {
        /// Id of the process.
        pid: u32,
        /// Underlying OS error.
        source: io::Error,
    },

    /// A memory signature was not found.
    ///
    /// This is normal for a short time after osu! starts, while it is still
    /// loading; retry after a moment.
    PatternNotFound {
        /// Internal name of the signature.
        name: &'static str,
    },

    /// Reading the process memory failed.
    ///
    /// Isolated failures are expected while the game swaps objects in memory
    /// (for example, while changing beatmaps); just read again. A
    /// [`PermissionDenied`](io::ErrorKind::PermissionDenied) source means the
    /// process cannot be read at all with the current privileges.
    ReadMemory {
        /// Address that was being read.
        address: u64,
        /// Number of bytes requested.
        size: usize,
        /// Underlying OS error.
        source: io::Error,
    },

    /// The game installation folder could not be determined.
    ///
    /// Set it with [`OsuReader::set_game_folder`](crate::OsuReader::set_game_folder)
    /// or the `OSU_PATH` environment variable.
    GameFolderNotFound,

    /// The `.osu` file doesn't exist or couldn't be parsed. Only returned by
    /// `attributes` (`difficulty` feature).
    Parse {
        /// Path of the `.osu` file.
        path: PathBuf,
        /// Underlying error; [`NotFound`](io::ErrorKind::NotFound) if the file
        /// doesn't exist.
        source: io::Error,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProcessNotFound => write!(f, "osu! is not running"),
            Self::OpenProcess { pid, source } => write!(f, "could not open osu! process {pid}: {source}"),
            Self::PatternNotFound { name } => write!(f, "memory signature `{name}` not found"),
            Self::ReadMemory { address, size, source } => {
                write!(f, "could not read {size} bytes at {address:#x}: {source}")
            }
            Self::GameFolderNotFound => write!(f, "could not determine the osu! installation folder"),
            Self::Parse { path, source } => write!(f, "could not parse {}: {source}", path.display()),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::OpenProcess { source, .. } | Self::ReadMemory { source, .. } | Self::Parse { source, .. } => {
                Some(source)
            }
            _ => None,
        }
    }
}

/// Shorthand for `Result<T, osu_memory_reader::Error>`.
pub type Result<T> = std::result::Result<T, Error>;
