// SPDX-License-Identifier: LGPL-3.0-only
// Copyright (C) 2026 Manolo-dl
//
// Based on tosu v4.26.3 (https://github.com/tosuapp/tosu):
//   packages/tosu/src/memory/stable.ts
//   packages/common/enums/osu.ts
// Copyright (C) 2023-2026 Mikhail Babynichev and contributors.
// Modified in November 2026: ported from TypeScript to Rust.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::macros::raw_enum;
use crate::process::Process;
use crate::stable::{Addresses, Sig};
use crate::{Result, Ruleset, country};

raw_enum! {
    /// What the user is doing, as shown to others on Bancho.
    pub enum BanchoStatus {
        /// Idle.
        Idle = 0 => "idle",
        /// Away.
        Afk = 1 => "afk",
        /// Playing.
        Playing = 2 => "playing",
        /// Editing a beatmap.
        Editing = 3 => "editing",
        /// Modding a beatmap.
        Modding = 4 => "modding",
        /// In a multiplayer room.
        Multiplayer = 5 => "multiplayer",
        /// Watching someone.
        Watching = 6 => "watching",
        /// Unknown.
        Unknown = 7 => "unknown",
        /// Testing a beatmap.
        Testing = 8 => "testing",
        /// Submitting a beatmap.
        Submitting = 9 => "submitting",
        /// Paused.
        Paused = 10 => "paused",
        /// In the multiplayer lobby.
        Lobby = 11 => "lobby",
        /// Playing in multiplayer.
        Multiplaying = 12 => "multiplaying",
        /// Browsing osu!direct.
        OsuDirect = 13 => "osuDirect",
    }
}

raw_enum! {
    /// Connection state of the game with Bancho.
    pub enum LoginStatus {
        /// Reconnecting.
        Reconnecting = 0 => "reconnecting",
        /// Not logged in.
        Guest = 256 => "guest",
        /// Logging in.
        ReceivingData = 257 => "receivingData",
        /// Disconnected.
        Disconnected = 65537 => "disconnected",
        /// Logged in.
        Connected = 65793 => "connected",
    }
}

/// Profile of the logged-in user.
///
/// Statistics are the ones of [`play_mode`](Self::play_mode), the mode
/// selected in the game.
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(rename_all = "camelCase"))]
#[non_exhaustive]
pub struct User {
    /// User id (`0` when not logged in).
    pub id: i32,
    /// Username.
    pub name: String,
    /// osu! country id.
    pub country_id: i32,
    /// Two-letter country code (`ES`, `US`...); empty if unknown.
    pub country_code: String,
    /// Global rank.
    pub rank: i32,
    /// Performance points.
    pub pp: i32,
    /// Accuracy, in percent.
    pub accuracy: f64,
    /// Ranked score.
    pub ranked_score: i64,
    /// Play count.
    pub play_count: i32,
    /// Level, with progress as the fractional part.
    pub level: f32,
    /// Mode the statistics belong to.
    pub play_mode: Option<Ruleset>,
    /// Status shown to others.
    pub bancho_status: BanchoStatus,
    /// Connection state.
    pub login_status: LoginStatus,
    /// Profile colour as ARGB (`0xAARRGGBB`).
    pub background_colour: u32,
}

pub(crate) fn read(p: &Process, a: &Addresses) -> Result<Option<User>> {
    let profile = p.read_ptr_ptr(a.get(p, Sig::UserProfile)?)?;
    if profile == 0 {
        return Ok(None);
    }

    let country_id = p.read_i32(profile + 0x9c)?;
    Ok(Some(User {
        id: p.read_i32(profile + 0x70)?,
        name: p.read_cs_string_ptr(profile + 0x30)?,
        country_id,
        country_code: country::code(country_id).unwrap_or_default().to_string(),
        rank: p.read_i32(profile + 0x84)?,
        pp: p.read_i32(profile + 0x88)?,
        accuracy: p.read_f64(profile + 0x4)?,
        ranked_score: p.read_i64(profile + 0xc)?,
        play_count: p.read_i32(profile + 0x7c)?,
        level: p.read_f32(profile + 0x74)?,
        play_mode: Ruleset::from_id(p.read_i32(profile + 0x80)?),
        bancho_status: BanchoStatus::from_raw(i32::from(p.read_u8(profile + 0x8c)?)),
        login_status: LoginStatus::from_raw(p.read_ptr_ptr(a.get(p, Sig::LoginStatus)?)? as i32),
        background_colour: p.read_u32(profile + 0xac)?,
    }))
}
