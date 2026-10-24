// SPDX-License-Identifier: LGPL-3.0-only
// Copyright (C) 2026 Manolo-dl

//! Reads the selected beatmap and its files from a running **osu! stable**
//! process, by reading the game's memory.
//!
//! This is a Rust port of the stable memory reader in
//! [tosu](https://github.com/tosuapp/tosu), without the server: you get plain
//! Rust values and decide what to do with them.
//!
//! # Example
//!
//! ```no_run
//! use osu_memory_reader::OsuReader;
//!
//! let reader = OsuReader::attach()?;
//!
//! if let Some(beatmap) = reader.beatmap()? {
//!     println!("{} - {} [{}]", beatmap.artist, beatmap.title, beatmap.difficulty);
//!
//!     let paths = reader.beatmap_paths(&beatmap)?;
//!     println!(".osu: {}", paths.osu.display());
//!     if let Some(background) = &paths.background {
//!         println!("bg:   {}", background.display());
//!     }
//! }
//! # Ok::<(), osu_memory_reader::Error>(())
//! ```
//!
//! # Polling
//!
//! The reader does not run anything in the background. Call
//! [`OsuReader::beatmap`] periodically (every 100–500 ms is plenty) and
//! compare [`Beatmap::checksum`] to detect a new selection. See
//! `examples/watch.rs` for a complete loop that also handles the game
//! starting and closing.
//!
//! [`OsuReader`] is `Send + Sync`, so it can live in a background thread or in
//! shared application state.
//!
//! # Platforms and permissions
//!
//! - **Windows**: reads with `ReadProcessMemory`. If osu! runs as
//!   administrator, the reading program must too.
//! - **Linux** (osu! under Wine, e.g. osu-winello): reads with
//!   `process_vm_readv`. Depending on the kernel's `ptrace_scope` setting it
//!   may require `sudo` or `setcap cap_sys_ptrace=eip` on the binary.
//!
//! # Features
//!
//! - `serde`: derives `Serialize` and `Deserialize` on [`Beatmap`],
//!   [`BeatmapPaths`], [`Ruleset`], [`RankedStatus`] and, with
//!   `difficulty`, `BeatmapAttributes` and `Bpm`. Struct fields are renamed
//!   to camelCase, which suits JavaScript frontends (e.g. Tauri).
//! - `difficulty`: adds `attributes`, which reads the `.osu` file with
//!   [rosu-pp](https://docs.rs/rosu-pp) to get the star rating, BPM,
//!   original mode and key count. Off by default to keep the reader light.

#![warn(missing_docs)]

mod beatmap;
#[cfg(feature = "difficulty")]
mod difficulty;
mod error;
mod paths;
mod pattern;
mod process;
mod stable;

use std::env;
use std::path::PathBuf;

pub use beatmap::{Beatmap, RankedStatus, Ruleset};
#[cfg(feature = "difficulty")]
pub use difficulty::{BeatmapAttributes, Bpm, attributes, attributes_from_path};
pub use error::{Error, Result};
pub use paths::BeatmapPaths;

use process::Process;
use stable::Addresses;

/// Executable name of osu! stable.
pub const PROCESS_NAME: &str = "osu!.exe";

/// A connection to a running osu! stable process.
///
/// Created with [`attach`](Self::attach). It stays valid while the game runs;
/// once [`is_running`](Self::is_running) returns `false`, drop it and attach
/// again when the game is back.
pub struct OsuReader {
    pid: u32,
    process: Process,
    addresses: Addresses,
    game_folder: Option<PathBuf>,
}

impl OsuReader {
    /// Ids of the running osu! stable processes.
    pub fn find_processes() -> Vec<u32> {
        Process::find(&[PROCESS_NAME])
    }

    /// Attaches to the first osu! stable process found.
    ///
    /// This scans the game memory, which takes a moment (typically well under
    /// a second), so call it once and keep the reader.
    ///
    /// # Errors
    ///
    /// - [`Error::ProcessNotFound`] if osu! is not running.
    /// - [`Error::PatternNotFound`] if the game is still loading; retry later.
    /// - [`Error::OpenProcess`] or [`Error::ReadMemory`] if the process can't
    ///   be read, usually because of permissions.
    pub fn attach() -> Result<Self> {
        let pid = *Self::find_processes().first().ok_or(Error::ProcessNotFound)?;
        Self::attach_pid(pid)
    }

    /// Attaches to a specific process, e.g. one of [`find_processes`](Self::find_processes).
    ///
    /// # Errors
    ///
    /// Same as [`attach`](Self::attach), except [`Error::ProcessNotFound`].
    pub fn attach_pid(pid: u32) -> Result<Self> {
        let process = Process::open(pid)?;
        let addresses = Addresses::scan(&process)?;
        Ok(Self { pid, process, addresses, game_folder: None })
    }

    /// Id of the attached process.
    pub fn pid(&self) -> u32 {
        self.pid
    }

    /// Whether the attached process is still running.
    pub fn is_running(&self) -> bool {
        self.process.is_alive()
    }

    /// The beatmap currently selected, or `None` if the game hasn't loaded one yet.
    ///
    /// # Errors
    ///
    /// [`Error::ReadMemory`] if the memory couldn't be read. This can happen
    /// for a moment while the game changes beatmaps; read again.
    pub fn beatmap(&self) -> Result<Option<Beatmap>> {
        stable::beatmap(&self.process, &self.addresses)
    }

    /// osu! installation folder.
    ///
    /// In order of priority: the folder set with
    /// [`set_game_folder`](Self::set_game_folder), the `OSU_PATH`
    /// environment variable, and automatic detection (the executable's folder
    /// on Windows; the process working directory or osu-winello's `osupath`
    /// on Linux).
    pub fn game_folder(&self) -> Option<PathBuf> {
        self.game_folder
            .clone()
            .or_else(|| env::var_os("OSU_PATH").map(PathBuf::from))
            .or_else(|| self.process.game_folder())
    }

    /// Overrides the detected osu! installation folder.
    pub fn set_game_folder(&mut self, folder: impl Into<PathBuf>) {
        self.game_folder = Some(folder.into());
    }

    /// Full paths on disk of `beatmap`'s files and folders.
    ///
    /// The songs folder is read from the game settings each time, so changes
    /// to `BeatmapDirectory` are picked up. Under Wine, Windows paths such as
    /// `D:\Songs` are translated through the prefix's drive mappings.
    ///
    /// # Errors
    ///
    /// - [`Error::GameFolderNotFound`] if the game folder can't be determined.
    /// - [`Error::ReadMemory`] if the settings couldn't be read.
    pub fn beatmap_paths(&self, beatmap: &Beatmap) -> Result<BeatmapPaths> {
        let game = self.game_folder().ok_or(Error::GameFolderNotFound)?;
        let songs = stable::songs_folder(&self.process, &self.addresses)?;
        Ok(paths::resolve(&self.process, game, &songs, beatmap))
    }
}

impl std::fmt::Debug for OsuReader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OsuReader").field("pid", &self.pid).finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reader_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<OsuReader>();
    }

    #[test]
    fn ruleset_ids_round_trip() {
        for ruleset in [Ruleset::Osu, Ruleset::Taiko, Ruleset::Fruits, Ruleset::Mania] {
            assert_eq!(Ruleset::from_id(ruleset.id()), Some(ruleset));
        }
        assert_eq!(Ruleset::from_id(9), None);
    }

    #[test]
    fn ranked_status_from_raw() {
        assert_eq!(RankedStatus::from_raw(4), RankedStatus::Ranked);
        assert_eq!(RankedStatus::from_raw(3), RankedStatus::Unknown);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serializes_as_camel_case() {
        let beatmap = Beatmap {
            mode: Some(Ruleset::Mania),
            ranked_status: RankedStatus::NotSubmitted,
            ..Default::default()
        };
        let json = serde_json::to_value(&beatmap).unwrap();
        assert_eq!(json["mode"], "mania");
        assert_eq!(json["rankedStatus"], "notSubmitted");
        assert!(json.get("backgroundFilename").is_some());
    }
}
