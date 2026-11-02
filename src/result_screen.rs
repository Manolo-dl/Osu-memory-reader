// SPDX-License-Identifier: LGPL-3.0-only
// Copyright (C) 2026 Manolo-dl
//
// Based on tosu v4.26.3 (https://github.com/tosuapp/tosu):
//   packages/tosu/src/memory/stable.ts
//   packages/tosu/src/utils/converters.ts
// Copyright (C) 2023-2026 Mikhail Babynichev and contributors.
// Modified in November 2026: ported from TypeScript to Rust.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::gameplay::read_hits;
use crate::process::Process;
use crate::stable::{self, Addresses};
use crate::state::is_on;
use crate::{GameState, Hits, Mods, Result, Ruleset};

/// The score shown on the results screen.
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(rename_all = "camelCase"))]
#[non_exhaustive]
pub struct ResultScreen {
    /// Online score id (`0` if the score wasn't submitted).
    pub online_id: i64,
    /// Name of the player.
    pub player_name: String,
    /// Mode played.
    pub mode: Option<Ruleset>,
    /// Mods used.
    pub mods: Mods,
    /// Score.
    pub score: i32,
    /// Highest combo.
    pub max_combo: u32,
    /// Judgement counts. Use [`Hits::accuracy`] and [`Hits::grade`] for the
    /// accuracy and grade.
    pub hits: Hits,
    /// When the score was set, as milliseconds since the Unix epoch.
    pub timestamp_ms: i64,
}

/// Converts the two halves of a .NET `DateTime` to Unix milliseconds. The two
/// top bits hold the `DateTimeKind` and are dropped.
fn net_date_to_unix_ms(high: i32, low: i32) -> i64 {
    const TICKS_PER_MS: i64 = 10_000;
    const UNIX_EPOCH_TICKS: i64 = 621_355_968_000_000_000;

    let ticks = (i64::from(high & 0x3fff_ffff) << 32) | i64::from(low as u32);
    (ticks - UNIX_EPOCH_TICKS) / TICKS_PER_MS
}

pub(crate) fn read(p: &Process, a: &Addresses) -> Result<Option<ResultScreen>> {
    if !is_on(p, a, GameState::ResultScreen)? {
        return Ok(None);
    }
    let ruleset = stable::ruleset(p, a)?;
    if ruleset == 0 {
        return Ok(None);
    }
    let base = p.read_ptr(ruleset + 0x38)?;
    if base == 0 {
        return Ok(None);
    }

    Ok(Some(ResultScreen {
        online_id: p.read_i64(base + 0x4)?,
        player_name: p.read_cs_string_ptr(base + 0x28)?,
        mode: Ruleset::from_id(p.read_i32(base + 0x64)?),
        mods: Mods::from_bits(stable::read_mods(p, p.read_ptr(base + 0x1c)?)?),
        score: p.read_i32(base + 0x78)?,
        max_combo: p.read_u16(base + 0x68)?.into(),
        hits: read_hits(p, base)?,
        timestamp_ms: net_date_to_unix_ms(p.read_i32(base + 0xa4)?, p.read_i32(base + 0xa0)?),
    }))
}

#[cfg(test)]
mod tests {
    #[test]
    fn net_date() {
        // DateTime(2024, 1, 1, 0, 0, 0, DateTimeKind.Utc).ToBinary()
        let binary: i64 = 0x48dc_0a5c_9900_c000;
        let ms = super::net_date_to_unix_ms((binary >> 32) as i32, binary as i32);
        assert_eq!(ms, 1_704_067_200_000);
    }
}
