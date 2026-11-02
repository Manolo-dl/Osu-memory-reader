// SPDX-License-Identifier: LGPL-3.0-only
// Copyright (C) 2026 Manolo-dl
//
// Based on tosu v4.26.3 (https://github.com/tosuapp/tosu):
//   packages/tosu/src/memory/stable.ts
// Copyright (C) 2023-2026 Mikhail Babynichev and contributors.
// Modified in November 2026: ported from TypeScript to Rust.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::process::Process;
use crate::stable::{self, Addresses};
use crate::state::is_on;
use crate::{GameState, Hits, Mods, Result};

/// The leaderboard shown on the left during gameplay.
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(rename_all = "camelCase"))]
#[non_exhaustive]
pub struct Leaderboard {
    /// Whether it is shown (the player can hide it with Tab).
    pub visible: bool,
    /// The player's own entry, updated live.
    pub player: Option<LeaderboardEntry>,
    /// All entries, in leaderboard order. Includes the player's.
    pub entries: Vec<LeaderboardEntry>,
}

/// One score of the [`Leaderboard`].
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(rename_all = "camelCase"))]
#[non_exhaustive]
pub struct LeaderboardEntry {
    /// User id, `0` when unknown.
    pub user_id: i32,
    /// Name.
    pub name: String,
    /// Score.
    pub score: i32,
    /// Combo.
    pub combo: u32,
    /// Highest combo.
    pub max_combo: u32,
    /// Mods.
    pub mods: Mods,
    /// Judgement counts. Gekis and katus are always zero here.
    pub hits: Hits,
    /// Position, starting at 1.
    pub position: i32,
    /// Team in multiplayer (`0` none, `1` blue, `2` red).
    pub team: i32,
    /// Whether the player is still alive (multiplayer).
    pub is_passing: bool,
}

/// More than this many entries means the list pointer is stale.
const MAX_ENTRIES: i32 = 1_000;

pub(crate) fn read(p: &Process, a: &Addresses) -> Result<Option<Leaderboard>> {
    if !is_on(p, a, GameState::Play)? {
        return Ok(None);
    }
    let ruleset = stable::ruleset(p, a)?;
    if ruleset == 0 {
        return Ok(None);
    }
    let base = p.read_ptr(ruleset + 0x78)?;
    if base == 0 {
        return Ok(None);
    }
    let board = p.read_ptr(base + 0x24)?;
    if board == 0 {
        return Ok(None);
    }

    let player_base = p.read_ptr(board + 0x10)?;
    let visible = player_base != 0 && p.read_bool(p.read_ptr(player_base + 0x24)? + 0x20)?;
    let player = if player_base == 0 { None } else { read_entry(p, player_base)? };

    // List of entries: items array at +0x4, count at +0xC.
    let list = p.read_ptr(board + 0x4)?;
    let (items, count) = if list == 0 { (0, 0) } else { (p.read_ptr(list + 0x4)?, p.read_i32(list + 0xc)?) };

    let mut entries = Vec::new();
    if items != 0 && (1..=MAX_ENTRIES).contains(&count) {
        for i in 0..count as u64 {
            // Entries can be half-built while the list changes; stop there.
            match read_entry(p, p.read_ptr(items + 0x8 + 0x4 * i)?)? {
                Some(entry) => entries.push(entry),
                None => break,
            }
        }
    }

    Ok(Some(Leaderboard { visible, player, entries }))
}

fn read_entry(p: &Process, base: u64) -> Result<Option<LeaderboardEntry>> {
    if base == 0 {
        return Ok(None);
    }
    let score = p.read_ptr(base + 0x20)?;
    if score == 0 {
        return Ok(None);
    }

    let user = p.read_ptr(score + 0x48)?;
    let user_id = if user == 0 { 0 } else { p.read_i32(user + 0x70)? };

    Ok(Some(LeaderboardEntry {
        user_id,
        name: p.read_cs_string_ptr(base + 0x8)?,
        score: p.read_i32(base + 0x30)?,
        combo: p.read_u16(score + 0x94)?.into(),
        max_combo: p.read_u16(score + 0x68)?.into(),
        mods: Mods::from_bits(stable::read_mods(p, p.read_ptr(score + 0x1c)?)?),
        hits: Hits {
            n300: p.read_u16(score + 0x8a)?.into(),
            n100: p.read_u16(score + 0x88)?.into(),
            n50: p.read_u16(score + 0x8c)?.into(),
            n_miss: p.read_u16(score + 0x92)?.into(),
            ..Hits::default()
        },
        position: p.read_i32(base + 0x2c)?,
        team: p.read_i32(base + 0x40)?,
        is_passing: p.read_bool(base + 0x4b)?,
    }))
}
