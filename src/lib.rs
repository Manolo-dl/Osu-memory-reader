// SPDX-License-Identifier: LGPL-3.0-only
// Copyright (C) 2026 Manolo-dl

//! Reads live data from a running **osu! stable** process: the selected
//! beatmap and its files, the game state, the play in progress, results,
//! leaderboard, user profile, settings and tournament client data.
//!
//! This is a Rust port of the stable memory reader in
//! [tosu](https://github.com/tosuapp/tosu), without the server: you get plain
//! Rust values and decide what to do with them.
//!
//! # Example
//!
//! ```no_run
//! use osu_memory_reader::{GameState, OsuReader};
//!
//! let reader = OsuReader::attach()?;
//!
//! if let Some(beatmap) = reader.beatmap()? {
//!     println!("{} - {} [{}]", beatmap.artist, beatmap.title, beatmap.difficulty);
//!
//!     let paths = reader.beatmap_paths(&beatmap)?;
//!     println!(".osu: {}", paths.osu.display());
//! }
//!
//! if reader.status()? == GameState::Play {
//!     if let Some(play) = reader.gameplay()? {
//!         println!("{} {}x {:.2}%", play.score, play.combo, play.accuracy);
//!     }
//! }
//! # Ok::<(), osu_memory_reader::Error>(())
//! ```
//!
//! # What can be read
//!
//! Every piece of data has its own method on [`OsuReader`], which reads only
//! that from memory. Call the ones you need, as often as you need them.
//!
//! | Method | Data | Available |
//! |---|---|---|
//! | [`status`](OsuReader::status) | Current screen ([`GameState`]) | Always |
//! | [`play_time`](OsuReader::play_time), [`game_time`](OsuReader::game_time) | Song position, game clock | Always |
//! | [`menu_mods`](OsuReader::menu_mods) | Mods selected in song select | Always |
//! | [`beatmap`](OsuReader::beatmap), [`beatmap_paths`](OsuReader::beatmap_paths) | Selected beatmap and its files | A beatmap is loaded |
//! | [`audio_length`](OsuReader::audio_length) | Song length | A song is loaded |
//! | [`audio_spectrum`](OsuReader::audio_spectrum) | Live spectrum of the music | [`GameState::Menu`] |
//! | [`user`](OsuReader::user) | Logged-in user's profile | Always |
//! | [`gameplay`](OsuReader::gameplay) | Score, combo, judgements, HP | [`GameState::Play`] |
//! | [`key_overlay`](OsuReader::key_overlay), [`hit_errors`](OsuReader::hit_errors) | Key presses, timing errors | [`GameState::Play`] |
//! | [`leaderboard`](OsuReader::leaderboard) | In-game leaderboard | [`GameState::Play`] |
//! | [`result_screen`](OsuReader::result_screen) | The finished score | [`GameState::ResultScreen`] |
//! | [`settings`](OsuReader::settings), [`mania_scroll_speed`](OsuReader::mania_scroll_speed) | Settings and keybinds | Always |
//! | [`skin_folder`](OsuReader::skin_folder), [`chat_status`](OsuReader::chat_status), [`is_interface_visible`](OsuReader::is_interface_visible), [`is_watching_replay`](OsuReader::is_watching_replay) | UI state | Always |
//! | [`tourney`](OsuReader::tourney), [`tourney_chat`](OsuReader::tourney_chat) | Tournament manager | [`GameState::Tourney`] (manager client) |
//! | [`spectating_user`](OsuReader::spectating_user) | Watched player | Tournament spectator clients |
//!
//! Methods tied to a screen check [`status`](OsuReader::status) themselves
//! and return `Ok(None)` when the game isn't on it: the game reuses those
//! objects for other screens, so reading them elsewhere would give garbage.
//!
//! Helpers that tosu computes on top of the raw values are available too:
//! [`Hits::accuracy`], [`Hits::grade`], [`unstable_rate`] and [`BassDensity`].
//!
//! # Polling
//!
//! The reader does not run anything in the background. Call the methods
//! periodically (every 100–500 ms is plenty for most data; tosu reads the
//! fast-changing ones every few milliseconds) and compare
//! [`Beatmap::checksum`] to detect a new selection. See `examples/watch.rs`
//! for a loop that also handles the game starting and closing, and
//! `examples/dump.rs` for every method at once.
//!
//! [`OsuReader`] is `Send + Sync`, so it can live in a background thread or in
//! shared application state.
//!
//! # Errors
//!
//! [`Error::ReadMemory`] and [`Error::Inconsistent`] can happen for a moment
//! while the game moves objects in memory (e.g. while changing screens): read
//! again.
//! [`Error::PatternNotFound`] means the memory signature that method needs
//! wasn't found: some only exist in certain clients (the tournament ones) or
//! after the game has used that feature once. The next call that needs a
//! missing signature searches the game's code memory again, which blocks it
//! for a fraction of a second (around 0.1 s); after a failed search the next
//! one waits 5 seconds, doubling up to a minute, so polling such a method
//! stays cheap.
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
//! - `serde`: derives `Serialize` and `Deserialize` on all the data types
//!   and, with `difficulty`, `BeatmapAttributes` and `Bpm`. Struct fields are
//!   renamed to camelCase, which suits JavaScript frontends (e.g. Tauri).
//! - `difficulty`: adds `attributes`, which reads the `.osu` file with
//!   [rosu-pp](https://docs.rs/rosu-pp) to get the star rating, BPM,
//!   original mode and key count. Off by default to keep the reader light.

#![warn(missing_docs)]

mod macros;

mod audio;
mod beatmap;
mod country;
#[cfg(feature = "difficulty")]
mod difficulty;
mod error;
mod gameplay;
mod leaderboard;
mod mods;
mod paths;
mod pattern;
mod process;
mod result_screen;
mod settings;
mod stable;
mod state;
mod tourney;
mod user;

use std::env;
use std::path::PathBuf;

pub use audio::{BassDensity, SPECTRUM_BANDS};
pub use beatmap::{Beatmap, RankedStatus, Ruleset};
#[cfg(feature = "difficulty")]
pub use difficulty::{BeatmapAttributes, Bpm, attributes, attributes_from_path};
pub use error::{Error, Result};
pub use gameplay::{Gameplay, Grade, Hits, KeyOverlay, KeyState, unstable_rate};
pub use leaderboard::{Leaderboard, LeaderboardEntry};
pub use mods::Mods;
pub use paths::BeatmapPaths;
pub use result_screen::ResultScreen;
pub use settings::{
    AudioSettings, BackgroundSettings, ClientSettings, CursorSettings, GroupType, Keybinds, LeaderboardType,
    ManiaSettings, MouseSettings, ProgressBarType, ReleaseStream, ResolutionSettings, ScoreMeterSettings,
    ScoreMeterType, Settings, SkinSettings, SortType, VirtualKey,
};
pub use state::{ChatStatus, GameState};
pub use tourney::{ChatMessage, Tourney, TourneyTeam, TourneyUser};
pub use user::{BanchoStatus, LoginStatus, User};

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
    /// Ids of the running osu! stable processes. A tournament has one per
    /// client (the manager and each spectator).
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

    // --- Game state ---

    /// Screen the game is on.
    pub fn status(&self) -> Result<GameState> {
        state::status(&self.process, &self.addresses)
    }

    /// Position of the song in milliseconds. Negative during the lead-in.
    pub fn play_time(&self) -> Result<i32> {
        state::play_time(&self.process, &self.addresses)
    }

    /// The game's internal clock in milliseconds.
    pub fn game_time(&self) -> Result<i32> {
        state::game_time(&self.process, &self.addresses)
    }

    /// Mods selected in song select. During a play, use
    /// [`Gameplay::mods`] instead: it also reflects multiplayer and replays.
    pub fn menu_mods(&self) -> Result<Mods> {
        state::menu_mods(&self.process, &self.addresses)
    }

    /// Visibility of the chat.
    pub fn chat_status(&self) -> Result<ChatStatus> {
        state::chat_status(&self.process, &self.addresses)
    }

    /// Whether the gameplay interface is shown (toggled with Shift+Tab).
    pub fn is_interface_visible(&self) -> Result<bool> {
        state::is_interface_visible(&self.process, &self.addresses)
    }

    /// Whether the play on screen is a replay.
    pub fn is_watching_replay(&self) -> Result<bool> {
        state::is_watching_replay(&self.process, &self.addresses)
    }

    /// Whether the replay interface is hidden. Always `false` outside replays.
    pub fn is_replay_ui_hidden(&self) -> Result<bool> {
        state::is_replay_ui_hidden(&self.process, &self.addresses)
    }

    /// Folder name of the skin in use, inside the game's `Skins` folder.
    pub fn skin_folder(&self) -> Result<String> {
        state::skin_folder(&self.process, &self.addresses)
    }

    // --- Beatmap ---

    /// The beatmap currently selected, or `None` if the game hasn't loaded one yet.
    ///
    /// # Errors
    ///
    /// [`Error::ReadMemory`] if the memory couldn't be read, or
    /// [`Error::Inconsistent`] if the game was moving the beatmap in memory
    /// and what was read is not valid. Both happen for a moment while the game
    /// changes beatmaps or screens; read again.
    pub fn beatmap(&self) -> Result<Option<Beatmap>> {
        beatmap::read(&self.process, &self.addresses)
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

    /// Full path of the songs folder (`BeatmapDirectory` in the settings).
    ///
    /// # Errors
    ///
    /// - [`Error::GameFolderNotFound`] if the game folder can't be determined.
    /// - [`Error::ReadMemory`] if the settings couldn't be read.
    pub fn songs_folder(&self) -> Result<PathBuf> {
        let game = self.game_folder().ok_or(Error::GameFolderNotFound)?;
        let songs = state::songs_folder(&self.process, &self.addresses)?;
        Ok(paths::songs_folder(&self.process, &game, &songs))
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
        let songs = state::songs_folder(&self.process, &self.addresses)?;
        Ok(paths::resolve(&self.process, game, &songs, beatmap))
    }

    /// Length of the loaded song in milliseconds.
    pub fn audio_length(&self) -> Result<f64> {
        state::audio_length(&self.process, &self.addresses)
    }

    /// Live spectrum of the music, in [`SPECTRUM_BANDS`] bands from low to
    /// high frequencies. Feed it to [`BassDensity`] for tosu's bass density.
    /// `None` outside [`GameState::Menu`], the only screen that exposes it.
    pub fn audio_spectrum(&self) -> Result<Option<[f32; SPECTRUM_BANDS]>> {
        audio::read_spectrum(&self.process, &self.addresses)
    }

    // --- User ---

    /// Profile of the logged-in user, or `None` before the game creates it.
    pub fn user(&self) -> Result<Option<User>> {
        user::read(&self.process, &self.addresses)
    }

    // --- Play ---

    /// The play in progress, including replays and spectating. `None`
    /// outside [`GameState::Play`].
    pub fn gameplay(&self) -> Result<Option<Gameplay>> {
        gameplay::read(&self.process, &self.addresses)
    }

    /// Key presses of the play in progress. `None` outside
    /// [`GameState::Play`], and in taiko and mania, which have no key
    /// overlay.
    pub fn key_overlay(&self) -> Result<Option<KeyOverlay>> {
        gameplay::read_key_overlay(&self.process, &self.addresses)
    }

    /// Timing errors of the hits so far in the play, in milliseconds
    /// (negative is early), starting at index `start`.
    ///
    /// To read only new errors, pass the number you already have:
    ///
    /// ```no_run
    /// # let reader = osu_memory_reader::OsuReader::attach()?;
    /// let mut errors = Vec::new();
    /// if let Some(new) = reader.hit_errors(errors.len())? {
    ///     errors.extend(new);
    /// }
    /// # Ok::<(), osu_memory_reader::Error>(())
    /// ```
    ///
    /// The list starts again with each play or retry. `None` outside
    /// [`GameState::Play`].
    pub fn hit_errors(&self, start: usize) -> Result<Option<Vec<i32>>> {
        gameplay::read_hit_errors(&self.process, &self.addresses, start)
    }

    /// The leaderboard on the left of the play. `None` outside
    /// [`GameState::Play`].
    pub fn leaderboard(&self) -> Result<Option<Leaderboard>> {
        leaderboard::read(&self.process, &self.addresses)
    }

    /// The score on the results screen. `None` outside
    /// [`GameState::ResultScreen`].
    pub fn result_screen(&self) -> Result<Option<ResultScreen>> {
        result_screen::read(&self.process, &self.addresses)
    }

    // --- Settings ---

    /// Game settings and keybinds.
    ///
    /// This reads a few hundred values, so poll it every few seconds at
    /// most, not every frame.
    pub fn settings(&self) -> Result<Settings> {
        settings::read(&self.process, &self.addresses)
    }

    /// osu!mania scroll speed in use. Unlike
    /// [`ManiaSettings::scroll_speed`], it reflects per-beatmap speeds.
    ///
    /// # Errors
    ///
    /// [`Error::PatternNotFound`] until the game has played a mania map.
    pub fn mania_scroll_speed(&self) -> Result<i32> {
        settings::mania_scroll_speed(&self.process, &self.addresses)
    }

    // --- Tournament ---

    /// State of the tournament manager client (`osu! -tournament`). `None`
    /// outside [`GameState::Tourney`].
    pub fn tourney(&self) -> Result<Option<Tourney>> {
        tourney::read(&self.process, &self.addresses)
    }

    /// Messages of the `#multiplayer` channel in the tournament manager.
    /// `None` outside [`GameState::Tourney`] or if the channel isn't open.
    /// Returns the whole channel each time.
    pub fn tourney_chat(&self) -> Result<Option<Vec<ChatMessage>>> {
        tourney::read_chat(&self.process, &self.addresses)
    }

    /// The user a tournament spectator client is watching, or `None` if its
    /// slot is empty.
    pub fn spectating_user(&self) -> Result<Option<TourneyUser>> {
        tourney::read_spectating_user(&self.process, &self.addresses)
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

    #[test]
    fn raw_enums_keep_unknown_values() {
        assert_eq!(GameState::from_raw(5), GameState::SelectPlay);
        assert_eq!(GameState::from_raw(99), GameState::Other(99));
        assert_eq!(GameState::Other(99).raw(), 99);
        assert_eq!(GameState::default(), GameState::Menu);
        assert_eq!(LoginStatus::from_raw(65793), LoginStatus::Connected);
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

    #[cfg(feature = "serde")]
    #[test]
    fn serializes_new_types() {
        assert_eq!(serde_json::to_value(GameState::SelectPlay).unwrap(), "selectPlay");
        assert_eq!(serde_json::to_value(GameState::Other(99)).unwrap(), serde_json::json!({ "other": 99 }));
        assert_eq!(
            serde_json::to_value(Mods::HIDDEN | Mods::DOUBLE_TIME).unwrap(),
            serde_json::json!({ "number": 72, "name": "HDDT" })
        );
        assert_eq!(serde_json::to_value(VirtualKey(90)).unwrap(), serde_json::json!({ "code": 90, "name": "Z" }));
        assert_eq!(serde_json::to_value(Grade::SH).unwrap(), "SH");

        let play = Gameplay { hits: Hits { n_geki: 3, ..Hits::default() }, ..Gameplay::default() };
        let json = serde_json::to_value(&play).unwrap();
        assert_eq!(json["hits"]["nGeki"], 3);
        assert!(json.get("hpSmooth").is_some());

        let back: Mods = serde_json::from_value(serde_json::json!({ "number": 72, "name": "HDDT" })).unwrap();
        assert_eq!(back, Mods::HIDDEN | Mods::DOUBLE_TIME);
    }
}
