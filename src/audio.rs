// SPDX-License-Identifier: LGPL-3.0-only
// Copyright (C) 2026 Manolo-dl
//
// Based on tosu v4.26.3 (https://github.com/tosuapp/tosu):
//   packages/tosu/src/memory/stable.ts
//   packages/tosu/src/states/bassDensity.ts
// Copyright (C) 2023-2026 Mikhail Babynichev and contributors.
// Modified in November 2026: ported from TypeScript to Rust.

use crate::process::Process;
use crate::stable::{self, Addresses};
use crate::Result;

/// Number of spectrum bands [`OsuReader::audio_spectrum`](crate::OsuReader::audio_spectrum) returns.
pub const SPECTRUM_BANDS: usize = 40;

pub(crate) fn read_spectrum(p: &Process, a: &Addresses) -> Result<Option<[f32; SPECTRUM_BANDS]>> {
    let ruleset = stable::ruleset(p, a)?;
    if ruleset == 0 {
        return Ok(None);
    }
    // [[Ruleset + 0x44] + 0x10]: float array with the FFT of the music.
    let array = p.read_ptr(p.read_ptr(ruleset + 0x44)? + 0x10)?;
    if array == 0 || p.read_i32(array + 0x4)? < SPECTRUM_BANDS as i32 {
        return Ok(None);
    }
    let values = p.read_f32_slice(array + 0x8, SPECTRUM_BANDS)?;
    Ok(values.try_into().ok())
}

/// Smoothed "bass density" of the music, as tosu computes it for overlays
/// that pulse with the beat.
///
/// Feed it every reading of
/// [`OsuReader::audio_spectrum`](crate::OsuReader::audio_spectrum), at a
/// steady rate (tosu reads it every 100 ms or so):
///
/// ```no_run
/// use osu_memory_reader::{BassDensity, OsuReader};
///
/// let reader = OsuReader::attach()?;
/// let mut bass = BassDensity::new();
/// if let Some(spectrum) = reader.audio_spectrum()? {
///     println!("density {:.2}", bass.update(&spectrum));
/// }
/// # Ok::<(), osu_memory_reader::Error>(())
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct BassDensity {
    velocity: f64,
}

impl BassDensity {
    /// Starts with no previous readings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a spectrum reading and returns the new density, from 0.5 (silence)
    /// up to 3.5.
    pub fn update(&mut self, spectrum: &[f32; SPECTRUM_BANDS]) -> f64 {
        let mut bass = 0.0;
        for (i, &value) in spectrum.iter().enumerate() {
            if value < 0.0 {
                return 0.5;
            }
            bass += 2.0 * f64::from(value) * (SPECTRUM_BANDS - i) as f64 / SPECTRUM_BANDS as f64;
        }
        if bass.is_nan() || self.velocity.is_nan() {
            self.velocity = 0.0;
            return 0.5;
        }

        self.velocity = self.velocity.max((bass * 1.5).min(6.0)) * 0.95;
        (1.0 + self.velocity) * 0.5
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn density() {
        let mut bass = BassDensity::new();
        assert_eq!(bass.update(&[0.0; SPECTRUM_BANDS]), 0.5);
        let loud = bass.update(&[1.0; SPECTRUM_BANDS]);
        assert!(loud > 3.0);
        // Decays when the music gets quiet.
        assert!(bass.update(&[0.0; SPECTRUM_BANDS]) < loud);
    }
}
