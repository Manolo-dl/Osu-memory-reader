// SPDX-License-Identifier: LGPL-3.0-only
// Copyright (C) 2026 Manolo-dl
//
// Based on tosu v4.26.3 (https://github.com/tosuapp/tosu):
//   packages/tosu/src/memory/stable.ts
//   packages/tosu/src/utils/calculators.ts
//   packages/tosu/src/states/gameplay.ts
// Copyright (C) 2023-2026 Mikhail Babynichev and contributors.
// Modified in November 2026: ported from TypeScript to Rust.

use std::fmt;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::process::Process;
use crate::stable::{self, Addresses, Sig};
use crate::state::is_on;
use crate::{GameState, Mods, Result, Ruleset};

/// Judgement counts, with osu! stable's names.
///
/// In taiko, catch and mania `geki` and `katu` have their own meaning:
/// MAX and 200 in mania, droplet misses in catch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(rename_all = "camelCase"))]
#[non_exhaustive]
pub struct Hits {
    /// 300s (GREAT).
    pub n300: u32,
    /// 100s (OK; 200 in catch for large droplets).
    pub n100: u32,
    /// 50s (MEH; small droplets in catch).
    pub n50: u32,
    /// Gekis (MAX in mania).
    pub n_geki: u32,
    /// Katus (200 in mania, missed droplets in catch).
    pub n_katu: u32,
    /// Misses.
    pub n_miss: u32,
}

impl Hits {
    /// Accuracy in percent, with osu! stable's formula for `mode`.
    /// `mods` only matters in mania, where ScoreV2 weights MAX higher.
    /// Returns 100 when there are no judgements yet, like the game.
    pub fn accuracy(&self, mode: Ruleset, mods: Mods) -> f64 {
        let [n300, n100, n50, geki, katu, miss] =
            [self.n300, self.n100, self.n50, self.n_geki, self.n_katu, self.n_miss].map(f64::from);

        let (numerator, denominator) = match mode {
            Ruleset::Osu => (6.0 * n300 + 2.0 * n100 + n50, 6.0 * (n300 + n100 + n50 + miss)),
            Ruleset::Taiko => (2.0 * n300 + n100, 2.0 * (n300 + n100 + miss)),
            Ruleset::Fruits => (n300 + n100 + n50, n300 + n100 + n50 + katu + miss),
            Ruleset::Mania => {
                let max_weight = if mods.contains(Mods::SCORE_V2) { 61.0 } else { 60.0 };
                (
                    max_weight * geki + 60.0 * n300 + 40.0 * katu + 20.0 * n100 + 10.0 * n50,
                    max_weight * (geki + n300 + katu + n100 + n50 + miss),
                )
            }
        };

        if denominator == 0.0 { 100.0 } else { numerator / denominator * 100.0 }
    }

    /// Grade with osu! stable's rules for `mode`. HD and FL give the silver
    /// grades.
    pub fn grade(&self, mode: Ruleset, mods: Mods) -> Grade {
        let silver = mods.contains(Mods::HIDDEN) || mods.contains(Mods::FLASHLIGHT);
        let (ss, s) = if silver { (Grade::XH, Grade::SH) } else { (Grade::X, Grade::S) };

        match mode {
            Ruleset::Osu | Ruleset::Taiko => {
                let total = self.n300 + self.n100 + self.n50 + self.n_miss;
                if total == 0 {
                    return ss;
                }
                let r300 = f64::from(self.n300) / f64::from(total);
                let r50 = f64::from(self.n50) / f64::from(total);
                let no_miss = self.n_miss == 0;

                if r300 == 1.0 {
                    ss
                } else if r300 > 0.9 && r50 < 0.01 && no_miss {
                    s
                } else if (r300 > 0.8 && no_miss) || r300 > 0.9 {
                    Grade::A
                } else if (r300 > 0.7 && no_miss) || r300 > 0.8 {
                    Grade::B
                } else if r300 > 0.6 {
                    Grade::C
                } else {
                    Grade::D
                }
            }
            Ruleset::Fruits | Ruleset::Mania => {
                let accuracy = self.accuracy(mode, mods) / 100.0;
                let [s_min, a_min, b_min, c_min] =
                    if mode == Ruleset::Fruits { [0.98, 0.94, 0.9, 0.85] } else { [0.95, 0.9, 0.8, 0.7] };

                if accuracy == 1.0 {
                    ss
                } else if accuracy > s_min {
                    s
                } else if accuracy > a_min {
                    Grade::A
                } else if accuracy > b_min {
                    Grade::B
                } else if accuracy > c_min {
                    Grade::C
                } else {
                    Grade::D
                }
            }
        }
    }
}

/// Grade of a play.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Grade {
    /// Silver SS.
    XH,
    /// SS.
    X,
    /// Silver S.
    SH,
    /// S.
    S,
    /// A.
    A,
    /// B.
    B,
    /// C.
    C,
    /// D.
    D,
}

impl Grade {
    /// `XH`, `X`, `SH`, `S`, `A`, `B`, `C` or `D`.
    pub fn name(self) -> &'static str {
        match self {
            Self::XH => "XH",
            Self::X => "X",
            Self::SH => "SH",
            Self::S => "S",
            Self::A => "A",
            Self::B => "B",
            Self::C => "C",
            Self::D => "D",
        }
    }
}

impl fmt::Display for Grade {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// The play in progress (also while watching a replay or spectating).
///
/// The judgements and combo can hold the previous play's values until the
/// first hit object of the new one is reached; tosu ignores them before then.
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(rename_all = "camelCase"))]
#[non_exhaustive]
pub struct Gameplay {
    /// Name of the player.
    pub player_name: String,
    /// Mode being played.
    pub mode: Option<Ruleset>,
    /// Mods of the play. ScoreV2 is included when it is the win condition of
    /// a multiplayer room.
    pub mods: Mods,
    /// Score.
    pub score: i32,
    /// Accuracy in percent, as the game shows it.
    pub accuracy: f64,
    /// Current combo.
    pub combo: u32,
    /// Highest combo so far.
    pub max_combo: u32,
    /// Health, from 0 to 200.
    pub hp: f64,
    /// Health as the bar draws it (interpolated).
    pub hp_smooth: f64,
    /// Whether health reached zero.
    pub failed: bool,
    /// Retries of the current beatmap in a row.
    pub retries: i32,
    /// Judgement counts.
    pub hits: Hits,
}

/// One key of the key overlay.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(rename_all = "camelCase"))]
#[non_exhaustive]
pub struct KeyState {
    /// Whether it is held down.
    pub pressed: bool,
    /// Times it was pressed during the play.
    pub count: u32,
}

/// The key overlay shown on the right during gameplay. Only osu!standard and
/// catch have one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(rename_all = "camelCase"))]
#[non_exhaustive]
pub struct KeyOverlay {
    /// K1 (left in catch).
    pub k1: KeyState,
    /// K2 (right in catch).
    pub k2: KeyState,
    /// M1 (dash in catch).
    pub m1: KeyState,
    /// M2 (unused in catch).
    pub m2: KeyState,
}

/// Unstable rate of the given hit errors (see
/// [`OsuReader::hit_errors`](crate::OsuReader::hit_errors)): ten times their
/// standard deviation, divided by the clock rate of `mods` as tosu does.
pub fn unstable_rate(hit_errors: &[i32], mods: Mods) -> f64 {
    if hit_errors.is_empty() {
        return 0.0;
    }
    let n = hit_errors.len() as f64;
    let mean = hit_errors.iter().map(|&e| f64::from(e)).sum::<f64>() / n;
    let mean_square = hit_errors.iter().map(|&e| f64::from(e).powi(2)).sum::<f64>() / n;
    (mean_square - mean * mean).max(0.0).sqrt() * 10.0 / mods.clock_rate()
}

/// Pointers of the play in progress: (ruleset, gameplay base, score base).
fn play_bases(p: &Process, a: &Addresses) -> Result<Option<(u64, u64, u64)>> {
    if !is_on(p, a, GameState::Play)? {
        return Ok(None);
    }
    let ruleset = stable::ruleset(p, a)?;
    if ruleset == 0 {
        return Ok(None);
    }
    let gameplay_base = p.read_ptr(ruleset + 0x64)?;
    if gameplay_base == 0 {
        return Ok(None);
    }
    let score_base = p.read_ptr(gameplay_base + 0x38)?;
    if score_base == 0 {
        return Ok(None);
    }
    Ok(Some((ruleset, gameplay_base, score_base)))
}

/// Judgements of a score object (gameplay, results and leaderboard share the layout).
pub(crate) fn read_hits(p: &Process, score: u64) -> Result<Hits> {
    Ok(Hits {
        n100: p.read_u16(score + 0x88)?.into(),
        n300: p.read_u16(score + 0x8a)?.into(),
        n50: p.read_u16(score + 0x8c)?.into(),
        n_geki: p.read_u16(score + 0x8e)?.into(),
        n_katu: p.read_u16(score + 0x90)?.into(),
        n_miss: p.read_u16(score + 0x92)?.into(),
    })
}

pub(crate) fn read(p: &Process, a: &Addresses) -> Result<Option<Gameplay>> {
    let Some((ruleset, gameplay_base, score_base)) = play_bases(p, a)? else {
        return Ok(None);
    };
    let hp_bar = p.read_ptr(gameplay_base + 0x40)?;
    if hp_bar == 0 {
        return Ok(None);
    }

    let score_processor = p.read_ptr(score_base + 0x54)?;
    let mut mods = Mods::from_bits(stable::read_mods(p, p.read_ptr(score_base + 0x1c)?)?);
    // ScoreV2 is not in the mods when it is the room's win condition.
    if score_processor != 0 {
        mods |= Mods::SCORE_V2;
    }
    let score = if score_processor == 0 { p.read_i32(score_base + 0x78)? } else { p.read_i32(ruleset + 0xf8)? };

    let hp = p.read_f64(hp_bar + 0x1c)?;
    let hp_smooth = p.read_f64(hp_bar + 0x14)?;
    let base = a.get(p, Sig::Base)?;

    Ok(Some(Gameplay {
        player_name: p.read_cs_string_ptr(score_base + 0x28)?,
        mode: Ruleset::from_id(p.read_i32(score_base + 0x64)?),
        mods,
        score,
        accuracy: p.read_f64(p.read_ptr(gameplay_base + 0x48)? + 0xc)?,
        combo: p.read_u16(score_base + 0x94)?.into(),
        max_combo: p.read_u16(score_base + 0x68)?.into(),
        hp,
        hp_smooth: if hp_smooth.is_finite() { hp_smooth } else { 0.0 },
        failed: hp <= 0.0,
        retries: p.read_i32(p.read_ptr(base - 0x33)? + 0x8)?,
        hits: read_hits(p, score_base)?,
    }))
}

pub(crate) fn read_key_overlay(p: &Process, a: &Addresses) -> Result<Option<KeyOverlay>> {
    if !is_on(p, a, GameState::Play)? {
        return Ok(None);
    }
    let ruleset = stable::ruleset(p, a)?;
    if ruleset == 0 {
        return Ok(None);
    }
    let overlay = p.read_ptr(ruleset + 0xac)?;
    if overlay == 0 {
        return Ok(None);
    }
    // [[Overlay + 0x10] + 0x4]: array of the four buttons.
    let buttons = p.read_ptr(p.read_ptr(overlay + 0x10)? + 0x4)?;
    if buttons == 0 || p.read_i32(buttons + 0x4)? < 4 {
        return Ok(None);
    }

    let key = |i: u64| -> Result<KeyState> {
        let button = p.read_ptr(buttons + 0x8 + 0x4 * i)?;
        Ok(KeyState { pressed: p.read_bool(button + 0x1c)?, count: p.read_i32(button + 0x14)?.max(0) as u32 })
    };
    Ok(Some(KeyOverlay { k1: key(0)?, k2: key(1)?, m1: key(2)?, m2: key(3)? }))
}

/// More than this many errors means the list pointer is stale.
const MAX_HIT_ERRORS: i32 = 100_000;

pub(crate) fn read_hit_errors(p: &Process, a: &Addresses, start: usize) -> Result<Option<Vec<i32>>> {
    let Some((_, _, score_base)) = play_bases(p, a)? else {
        return Ok(None);
    };
    // List<int>: items array at +0x4, count at +0xC.
    let list = p.read_ptr(score_base + 0x38)?;
    if list == 0 {
        return Ok(None);
    }
    let items = p.read_ptr(list + 0x4)?;
    let count = p.read_i32(list + 0xc)?;
    if items == 0 || !(0..=MAX_HIT_ERRORS).contains(&count) {
        return Ok(None);
    }

    let count = count as usize;
    if start >= count {
        return Ok(Some(Vec::new()));
    }
    let errors = p.read_i32_slice(items + 0x8 + 0x4 * start as u64, count - start)?;
    // Values far outside any hit window are garbage from a stale list.
    Ok(Some(errors.into_iter().take_while(|e| e.abs() <= 10_000).collect()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hits(n300: u32, n100: u32, n50: u32, n_geki: u32, n_katu: u32, n_miss: u32) -> Hits {
        Hits { n300, n100, n50, n_geki, n_katu, n_miss }
    }

    #[test]
    fn accuracy() {
        assert_eq!(Hits::default().accuracy(Ruleset::Osu, Mods::NONE), 100.0);
        let osu = hits(90, 10, 0, 0, 0, 0).accuracy(Ruleset::Osu, Mods::NONE);
        assert!((osu - 93.333).abs() < 0.001);
        let mania = hits(0, 0, 0, 10, 0, 0);
        assert_eq!(mania.accuracy(Ruleset::Mania, Mods::NONE), 100.0);
        let mania = hits(10, 0, 0, 0, 10, 0).accuracy(Ruleset::Mania, Mods::NONE);
        assert!((mania - 83.333).abs() < 0.001);
    }

    #[test]
    fn grades() {
        assert_eq!(hits(100, 0, 0, 0, 0, 0).grade(Ruleset::Osu, Mods::HIDDEN), Grade::XH);
        assert_eq!(hits(95, 5, 0, 0, 0, 0).grade(Ruleset::Osu, Mods::NONE), Grade::S);
        assert_eq!(hits(95, 4, 0, 0, 0, 1).grade(Ruleset::Osu, Mods::NONE), Grade::A);
        assert_eq!(hits(0, 0, 0, 100, 0, 0).grade(Ruleset::Mania, Mods::NONE), Grade::X);
        // (80 * 60 + 20 * 20) / (100 * 60) = 86.7%
        assert_eq!(hits(80, 20, 0, 0, 0, 0).grade(Ruleset::Mania, Mods::NONE), Grade::B);
    }

    #[test]
    fn unstable_rate_matches_std_dev() {
        assert_eq!(unstable_rate(&[], Mods::NONE), 0.0);
        assert_eq!(unstable_rate(&[-10, 10], Mods::NONE), 100.0);
        assert_eq!(unstable_rate(&[-10, 10], Mods::DOUBLE_TIME), 100.0 / 1.5);
    }
}
