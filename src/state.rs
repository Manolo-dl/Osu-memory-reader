// SPDX-License-Identifier: LGPL-3.0-only
// Copyright (C) 2026 Manolo-dl
//
// Based on tosu v4.26.3 (https://github.com/tosuapp/tosu):
//   packages/common/enums/osu.ts
//   packages/tosu/src/memory/stable.ts
// Copyright (C) 2023-2026 Mikhail Babynichev and contributors.
// Modified in November 2026: ported from TypeScript to Rust.

//! Global game state: screen, clocks, menu mods, skin and UI flags.

use crate::macros::raw_enum;
use crate::process::Process;
use crate::stable::{self, Addresses, Sig};
use crate::{Mods, Result};

raw_enum! {
    /// Screen the game is on.
    pub enum GameState {
        /// Main menu.
        Menu = 0 => "menu",
        /// Beatmap editor.
        Edit = 1 => "edit",
        /// Playing, watching a replay or spectating.
        Play = 2 => "play",
        /// The game is closing.
        Exit = 3 => "exit",
        /// Song select for the editor.
        SelectEdit = 4 => "selectEdit",
        /// Song select for playing.
        SelectPlay = 5 => "selectPlay",
        /// Song select for tournament drawings.
        SelectDrawings = 6 => "selectDrawings",
        /// Results screen.
        ResultScreen = 7 => "resultScreen",
        /// Updater.
        Update = 8 => "update",
        /// Loading.
        Busy = 9 => "busy",
        /// The game itself reports an unknown state.
        Unknown = 10 => "unknown",
        /// Multiplayer lobby.
        Lobby = 11 => "lobby",
        /// Multiplayer room.
        MatchSetup = 12 => "matchSetup",
        /// Song select inside a multiplayer room.
        SelectMulti = 13 => "selectMulti",
        /// Multiplayer results (head to head).
        RankingVs = 14 => "rankingVs",
        /// osu!direct.
        OnlineSelection = 15 => "onlineSelection",
        /// Offset wizard.
        OptionsOffsetWizard = 16 => "optionsOffsetWizard",
        /// Multiplayer results (tag co-op).
        RankingTagCoop = 17 => "rankingTagCoop",
        /// Multiplayer results (team VS).
        RankingTeam = 18 => "rankingTeam",
        /// Importing beatmaps.
        BeatmapImport = 19 => "beatmapImport",
        /// Package updater.
        PackageUpdater = 20 => "packageUpdater",
        /// Benchmark.
        Benchmark = 21 => "benchmark",
        /// Tournament client.
        Tourney = 22 => "tourney",
        /// Charts.
        Charts = 23 => "charts",
    }
}

raw_enum! {
    /// Visibility of the in-game chat.
    pub enum ChatStatus {
        /// Hidden.
        Hidden = 0 => "hidden",
        /// Visible.
        Visible = 1 => "visible",
        /// Visible with the friends list.
        VisibleWithFriendsList = 2 => "visibleWithFriendsList",
    }
}

pub(crate) fn status(p: &Process, a: &Addresses) -> Result<GameState> {
    Ok(GameState::from_raw(p.read_ptr_ptr(a.get(p, Sig::Status)?)? as i32))
}

/// Screen-specific objects are reused for other screens, so their readers
/// check the screen first.
pub(crate) fn is_on(p: &Process, a: &Addresses, screen: GameState) -> Result<bool> {
    Ok(status(p, a)? == screen)
}

pub(crate) fn play_time(p: &Process, a: &Addresses) -> Result<i32> {
    p.read_i32(p.read_ptr(a.get(p, Sig::PlayTime)? + 0x5)?)
}

pub(crate) fn game_time(p: &Process, a: &Addresses) -> Result<i32> {
    Ok(p.read_ptr_ptr(a.get(p, Sig::GameTime)?)? as i32)
}

pub(crate) fn menu_mods(p: &Process, a: &Addresses) -> Result<Mods> {
    Ok(Mods::from_bits(p.read_ptr_ptr(a.get(p, Sig::MenuMods)?)? as u32))
}

pub(crate) fn chat_status(p: &Process, a: &Addresses) -> Result<ChatStatus> {
    Ok(ChatStatus::from_raw(i32::from(p.read_u8(p.read_ptr(a.get(p, Sig::ChatChecker)?)?)?)))
}

pub(crate) fn is_interface_visible(p: &Process, a: &Addresses) -> Result<bool> {
    // [[SettingsClass + 0x8] + 0x4] + 0xC
    let settings = p.read_ptr(a.get(p, Sig::SettingsClass)? + 0x8)?;
    p.read_bool(p.read_ptr(settings + 0x4)? + 0xc)
}

pub(crate) fn is_watching_replay(p: &Process, a: &Addresses) -> Result<bool> {
    Ok(p.read_u8(p.read_ptr(a.get(p, Sig::CanRunSlowly)? + 0x46)?)? == 1)
}

pub(crate) fn is_replay_ui_hidden(p: &Process, a: &Addresses) -> Result<bool> {
    if !is_watching_replay(p, a)? {
        return Ok(false);
    }
    let ruleset = stable::ruleset(p, a)?;
    if ruleset == 0 {
        return Ok(false);
    }
    p.read_bool(ruleset + 0x1d8)
}

pub(crate) fn skin_folder(p: &Process, a: &Addresses) -> Result<String> {
    let skin_osu = p.read_ptr(a.get(p, Sig::SkinData)? + 0x7)?;
    if skin_osu == 0 {
        return Ok(String::new());
    }
    let skin_base = p.read_ptr(skin_osu)?;
    if skin_base == 0 {
        return Ok(String::new());
    }
    p.read_cs_string_ptr(skin_base + 0x44)
}

/// Songs folder exactly as written in the game settings (`BeatmapDirectory`).
pub(crate) fn songs_folder(p: &Process, a: &Addresses) -> Result<String> {
    // [[[SettingsClass + 0x8] + 0xB8] + 0x4]
    let settings = p.read_ptr(a.get(p, Sig::SettingsClass)? + 0x8)?;
    p.read_cs_string_ptr(p.read_ptr(settings + 0xb8)? + 0x4)
}

pub(crate) fn audio_length(p: &Process, a: &Addresses) -> Result<f64> {
    p.read_f64(p.read_ptr_ptr(a.get(p, Sig::AudioLength)?)? + 0x4)
}
