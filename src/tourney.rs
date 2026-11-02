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
use crate::stable::{self, Addresses, Sig};
use crate::state::is_on;
use crate::{GameState, Result};

/// State of the tournament manager (the `osu! -tournament` client).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(rename_all = "camelCase"))]
#[non_exhaustive]
pub struct Tourney {
    /// State the manager shares with the spectator clients over IPC.
    pub ipc_state: i32,
    /// Best of N.
    pub best_of: i32,
    /// Left (red) team.
    pub left: TourneyTeam,
    /// Right (blue) team.
    pub right: TourneyTeam,
    /// Whether the stars (points) are shown.
    pub stars_visible: bool,
    /// Whether the scores are shown.
    pub score_visible: bool,
}

/// One side of the [`Tourney`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(rename_all = "camelCase"))]
#[non_exhaustive]
pub struct TourneyTeam {
    /// Team name.
    pub name: String,
    /// Points won in the match.
    pub stars: i32,
    /// Score of the current map.
    pub score: i32,
}

/// A message of the `#multiplayer` channel in the tournament manager.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(rename_all = "camelCase"))]
#[non_exhaustive]
pub struct ChatMessage {
    /// Time as the chat shows it, e.g. `18:04`.
    pub time: String,
    /// Sender.
    pub name: String,
    /// Text. `!mp` commands are included.
    pub content: String,
}

/// The user a tournament spectator client is watching.
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(rename_all = "camelCase"))]
#[non_exhaustive]
pub struct TourneyUser {
    /// User id.
    pub id: i32,
    /// Username.
    pub name: String,
    /// Country code.
    pub country: String,
    /// Accuracy in percent.
    pub accuracy: f64,
    /// Ranked score.
    pub ranked_score: i64,
    /// Play count.
    pub play_count: i32,
    /// Global rank.
    pub global_rank: i32,
    /// Performance points.
    pub pp: i32,
}

pub(crate) fn read(p: &Process, a: &Addresses) -> Result<Option<Tourney>> {
    if !is_on(p, a, GameState::Tourney)? {
        return Ok(None);
    }
    let ruleset = stable::ruleset(p, a)?;
    if ruleset == 0 {
        return Ok(None);
    }
    let left = p.read_ptr(ruleset + 0x1c)?;
    let right = p.read_ptr(ruleset + 0x20)?;
    if left == 0 || right == 0 {
        return Ok(None);
    }

    let team = |base: u64| -> Result<TourneyTeam> {
        Ok(TourneyTeam {
            name: p.read_cs_string_ptr(p.read_ptr(base + 0x20)? + 0x144)?,
            stars: p.read_i32(base + 0x2c)?,
            score: p.read_i32(base + 0x28)?,
        })
    };

    Ok(Some(Tourney {
        ipc_state: p.read_i32(ruleset + 0x54)?,
        best_of: p.read_i32(right + 0x30)?,
        left: team(left)?,
        right: team(right)?,
        stars_visible: p.read_bool(right + 0x38)?,
        score_visible: p.read_bool(right + 0x39)?,
    }))
}

/// More than this many items means the list pointer is stale.
const MAX_ITEMS: i32 = 100_000;

pub(crate) fn read_chat(p: &Process, a: &Addresses) -> Result<Option<Vec<ChatMessage>>> {
    let channels = p.read_ptr_ptr(a.get(p, Sig::TourneyChat)? + 0x1)?;
    if channels == 0 {
        return Ok(None);
    }
    // Array of channels: length at +0x4, items from +0x8.
    let items = p.read_ptr(channels + 0x4)?;
    if items == 0 {
        return Ok(None);
    }
    let count = p.read_i32(items + 0x4)?;
    if !(0..=MAX_ITEMS).contains(&count) {
        return Ok(None);
    }

    for i in (0..count as u64).rev() {
        let channel = p.read_ptr(items + 0x8 + 0x4 * i)?;
        if channel == 0 || p.read_cs_string_ptr(channel + 0x4)? != "#multiplayer" {
            continue;
        }

        // List of messages: items array at +0x4, count at +0xC.
        let list = p.read_ptr(channel + 0x10)?;
        if list == 0 {
            return Ok(Some(Vec::new()));
        }
        let messages = p.read_ptr(list + 0x4)?;
        let count = p.read_i32(list + 0xc)?;
        if messages == 0 || !(0..=MAX_ITEMS).contains(&count) {
            return Ok(Some(Vec::new()));
        }

        let mut result = Vec::with_capacity(count as usize);
        for m in 0..count as u64 {
            let message = p.read_ptr(messages + 0x8 + 0x4 * m)?;
            let content = p.read_cs_string_ptr(message + 0x4)?;
            if content.is_empty() {
                continue;
            }
            // "18:04 name:" -> ("18:04", "name")
            let header = p.read_cs_string_ptr(message + 0x8)?;
            let (time, name) = header.split_once(' ').unwrap_or((&header, ""));
            result.push(ChatMessage {
                time: time.trim().to_string(),
                name: name.trim().trim_end_matches(':').to_string(),
                content,
            });
        }
        return Ok(Some(result));
    }

    Ok(None)
}

pub(crate) fn read_spectating_user(p: &Process, a: &Addresses) -> Result<Option<TourneyUser>> {
    let user = p.read_ptr_ptr(a.get(p, Sig::SpectatingUser)?)?;
    if user == 0 {
        return Ok(None);
    }

    Ok(Some(TourneyUser {
        id: p.read_i32(user + 0x70)?,
        name: p.read_cs_string_ptr(user + 0x30)?,
        country: p.read_cs_string_ptr(user + 0x2c)?,
        accuracy: p.read_f64(user + 0x4)?,
        ranked_score: p.read_i64(user + 0xc)?,
        play_count: p.read_i32(user + 0x7c)?,
        global_rank: p.read_i32(user + 0x84)?,
        pp: p.read_i32(user + 0x9c)?,
    }))
}
