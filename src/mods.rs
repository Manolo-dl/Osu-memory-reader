// SPDX-License-Identifier: LGPL-3.0-only
// Copyright (C) 2026 Manolo-dl
//
// Based on tosu v4.26.3 (https://github.com/tosuapp/tosu):
//   packages/tosu/src/utils/osuMods.ts
//   packages/tosu/src/utils/osuMods.types.ts
// Copyright (C) 2023-2026 Mikhail Babynichev and contributors.
// Modified in November 2026: ported from TypeScript to Rust.

use std::fmt;
use std::ops::{BitOr, BitOrAssign};

/// osu! stable mods, as the bit field the game stores.
///
/// ```
/// use osu_memory_reader::Mods;
///
/// let mods = Mods::HIDDEN | Mods::DOUBLE_TIME;
/// assert!(mods.contains(Mods::HIDDEN));
/// assert_eq!(mods.to_string(), "HDDT");
/// assert_eq!(mods.clock_rate(), 1.5);
/// ```
///
/// With the `serde` feature it is (de)serialized as
/// `{ "number": 72, "name": "HDDT" }`, like tosu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Mods(u32);

/// Acronym of each bit, from bit 0 upwards.
const ACRONYMS: [&str; 31] = [
    "NF", "EZ", "TD", "HD", "HR", "SD", "DT", "RX", "HT", "NC", "FL", "AT", "SO", "AP", "PF", "4K", "5K", "6K", "7K",
    "8K", "FI", "RD", "CN", "TG", "9K", "10K", "1K", "3K", "2K", "V2", "MR",
];

/// Display position of the mods tosu sorts; the rest go after them, in bit order.
fn display_order(acronym: &str) -> u8 {
    match acronym {
        "NF" => 0,
        "EZ" => 1,
        "HD" => 2,
        "DT" | "NC" | "HT" => 3,
        "HR" => 4,
        "SO" | "SD" | "PF" => 5,
        "FL" => 6,
        "TD" => 7,
        _ => 99,
    }
}

#[allow(missing_docs)]
impl Mods {
    pub const NONE: Self = Self(0);
    pub const NO_FAIL: Self = Self(1 << 0);
    pub const EASY: Self = Self(1 << 1);
    pub const TOUCH_DEVICE: Self = Self(1 << 2);
    pub const HIDDEN: Self = Self(1 << 3);
    pub const HARD_ROCK: Self = Self(1 << 4);
    pub const SUDDEN_DEATH: Self = Self(1 << 5);
    pub const DOUBLE_TIME: Self = Self(1 << 6);
    pub const RELAX: Self = Self(1 << 7);
    pub const HALF_TIME: Self = Self(1 << 8);
    /// Always set together with [`DOUBLE_TIME`](Self::DOUBLE_TIME).
    pub const NIGHTCORE: Self = Self(1 << 9);
    pub const FLASHLIGHT: Self = Self(1 << 10);
    pub const AUTOPLAY: Self = Self(1 << 11);
    pub const SPUN_OUT: Self = Self(1 << 12);
    pub const AUTOPILOT: Self = Self(1 << 13);
    /// Always set together with [`SUDDEN_DEATH`](Self::SUDDEN_DEATH).
    pub const PERFECT: Self = Self(1 << 14);
    pub const KEY4: Self = Self(1 << 15);
    pub const KEY5: Self = Self(1 << 16);
    pub const KEY6: Self = Self(1 << 17);
    pub const KEY7: Self = Self(1 << 18);
    pub const KEY8: Self = Self(1 << 19);
    pub const FADE_IN: Self = Self(1 << 20);
    pub const RANDOM: Self = Self(1 << 21);
    /// Always set together with [`AUTOPLAY`](Self::AUTOPLAY).
    pub const CINEMA: Self = Self(1 << 22);
    pub const TARGET: Self = Self(1 << 23);
    pub const KEY9: Self = Self(1 << 24);
    pub const KEY10: Self = Self(1 << 25);
    pub const KEY1: Self = Self(1 << 26);
    pub const KEY3: Self = Self(1 << 27);
    pub const KEY2: Self = Self(1 << 28);
    pub const SCORE_V2: Self = Self(1 << 29);
    pub const MIRROR: Self = Self(1 << 30);
}

impl Mods {
    /// Wraps the raw bit field.
    pub const fn from_bits(bits: u32) -> Self {
        Self(bits)
    }

    /// The raw bit field.
    pub const fn bits(self) -> u32 {
        self.0
    }

    /// Whether no mod is set.
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Whether every mod in `other` is set.
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Acronyms of the set mods, in the usual display order (`HD` before
    /// `DT`...). Implied mods are left out: `NC` instead of `DT NC`, `PF`
    /// instead of `SD PF`, `CN` instead of `AT CN`.
    pub fn acronyms(self) -> Vec<&'static str> {
        let mut bits = self.0;
        for (implying, implied) in
            [(Self::NIGHTCORE, Self::DOUBLE_TIME), (Self::PERFECT, Self::SUDDEN_DEATH), (Self::CINEMA, Self::AUTOPLAY)]
        {
            if self.contains(implying) {
                bits &= !implied.0;
            }
        }

        let mut acronyms: Vec<&str> =
            ACRONYMS.iter().enumerate().filter(|(i, _)| bits & (1 << i) != 0).map(|(_, a)| *a).collect();
        acronyms.sort_by_key(|a| display_order(a));
        acronyms
    }

    /// Speed multiplier: 1.5 with DT/NC, 0.75 with HT, 1 otherwise.
    pub const fn clock_rate(self) -> f64 {
        if self.contains(Self::DOUBLE_TIME) || self.contains(Self::NIGHTCORE) {
            1.5
        } else if self.contains(Self::HALF_TIME) {
            0.75
        } else {
            1.0
        }
    }
}

/// The acronyms joined (`HDDT`); empty when no mod is set, like tosu.
impl fmt::Display for Mods {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for acronym in self.acronyms() {
            f.write_str(acronym)?;
        }
        Ok(())
    }
}

impl BitOr for Mods {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for Mods {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl From<u32> for Mods {
    fn from(bits: u32) -> Self {
        Self(bits)
    }
}

#[cfg(feature = "serde")]
mod serde_impl {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    use super::Mods;

    #[derive(Serialize)]
    struct ModsOut {
        number: u32,
        name: String,
    }

    #[derive(Deserialize)]
    struct ModsIn {
        number: u32,
    }

    impl Serialize for Mods {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            ModsOut { number: self.0, name: self.to_string() }.serialize(serializer)
        }
    }

    impl<'de> Deserialize<'de> for Mods {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            ModsIn::deserialize(deserializer).map(|m| Mods(m.number))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names() {
        assert_eq!(Mods::NONE.to_string(), "");
        assert_eq!((Mods::DOUBLE_TIME | Mods::HIDDEN).to_string(), "HDDT");
        assert_eq!((Mods::DOUBLE_TIME | Mods::NIGHTCORE | Mods::HARD_ROCK).to_string(), "NCHR");
        assert_eq!((Mods::SUDDEN_DEATH | Mods::PERFECT).to_string(), "PF");
        assert_eq!((Mods::KEY4 | Mods::MIRROR | Mods::NO_FAIL).to_string(), "NF4KMR");
    }

    #[test]
    fn clock_rate() {
        assert_eq!(Mods::NONE.clock_rate(), 1.0);
        assert_eq!((Mods::DOUBLE_TIME | Mods::NIGHTCORE).clock_rate(), 1.5);
        assert_eq!(Mods::HALF_TIME.clock_rate(), 0.75);
    }
}
