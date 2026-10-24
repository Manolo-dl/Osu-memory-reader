// SPDX-License-Identifier: LGPL-3.0-only
// Copyright (C) 2026 Manolo-dl

//! Attributes computed from the `.osu` file with rosu-pp.

use std::path::Path;

use rosu_pp::model::mode::GameMode;
use rosu_pp::{Difficulty, GameMods};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{BeatmapPaths, Error, Result, Ruleset};

/// Beats per minute of a beatmap.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(rename_all = "camelCase"))]
#[non_exhaustive]
pub struct Bpm {
    /// BPM that lasts the longest, as shown in song select.
    pub common: f64,
    /// Lowest BPM of any timing point.
    pub min: f64,
    /// Highest BPM of any timing point.
    pub max: f64,
}

/// Attributes of a beatmap that need its `.osu` file, computed by
/// [`attributes`].
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(rename_all = "camelCase"))]
#[non_exhaustive]
pub struct BeatmapAttributes {
    /// Star rating without mods, in the mode the map is played in.
    pub stars: f64,
    /// BPM without mods.
    pub bpm: Bpm,
    /// Mode the map was made for (the `Mode:` of the `.osu` file).
    pub original_mode: Ruleset,
    /// Mode the map is played in: the requested one when the map can be
    /// converted to it, otherwise [`original_mode`](Self::original_mode).
    pub mode: Ruleset,
    /// Number of keys when played in mania, native or converted.
    pub key_count: Option<u32>,
}

/// Reads `paths.osu` and computes its star rating, BPM, original mode and key
/// count.
///
/// `mode` is the ruleset to compute for, usually the selected one
/// ([`Beatmap::mode`](crate::Beatmap::mode)). Only osu!standard maps can be
/// converted; maps made for another mode keep their own, as in osu! stable.
/// `None` uses the map's own mode.
///
/// Stars are calculated without mods, the way osu! stable does it.
///
/// This parses the whole `.osu` file and runs the difficulty calculation, so
/// call it when [`Beatmap::checksum`](crate::Beatmap::checksum) changes,
/// not on every poll.
///
/// # Errors
///
/// [`Error::Parse`] if the `.osu` file doesn't exist or can't be parsed.
///
/// # Example
///
/// ```no_run
/// use osu_memory_reader::{OsuReader, attributes};
///
/// let reader = OsuReader::attach()?;
/// if let Some(beatmap) = reader.beatmap()? {
///     let paths = reader.beatmap_paths(&beatmap)?;
///     let attrs = attributes(&paths, beatmap.mode)?;
///     println!("{:.2}★  {:.0} BPM  keys: {:?}", attrs.stars, attrs.bpm.common, attrs.key_count);
/// }
/// # Ok::<(), osu_memory_reader::Error>(())
/// ```
pub fn attributes(paths: &BeatmapPaths, mode: Option<Ruleset>) -> Result<BeatmapAttributes> {
    attributes_from_path(&paths.osu, mode)
}

/// Same as [`attributes`], for any `.osu` file.
///
/// # Errors
///
/// [`Error::Parse`] if the file doesn't exist or can't be parsed.
pub fn attributes_from_path(osu: impl AsRef<Path>, mode: Option<Ruleset>) -> Result<BeatmapAttributes> {
    let osu = osu.as_ref();
    let mut map = rosu_pp::Beatmap::from_path(osu).map_err(|source| Error::Parse { path: osu.to_path_buf(), source })?;

    let original_mode = ruleset(map.mode);
    if let Some(mode) = mode {
        // Fails for maps that aren't osu!standard; those stay in their own mode.
        let _ = map.convert_mut(game_mode(mode), &GameMods::default());
    }
    let mode = ruleset(map.mode);

    let stars = Difficulty::new().lazer(false).calculate(&map).stars();
    // Mania conversion stores the resulting key count in CS.
    let key_count = (mode == Ruleset::Mania).then(|| map.cs.round() as u32);

    Ok(BeatmapAttributes { stars, bpm: bpm(&map), original_mode, mode, key_count })
}

fn bpm(map: &rosu_pp::Beatmap) -> Bpm {
    let beat_lens = map.timing_points.iter().map(|t| t.beat_len);
    let (Some(shortest), Some(longest)) = (beat_lens.clone().reduce(f64::min), beat_lens.reduce(f64::max)) else {
        return Bpm::default();
    };

    Bpm {
        // Most common beat length weighted by duration, with the first
        // timing point starting at 0 like osu! stable.
        common: map.bpm(),
        min: 60_000.0 / longest,
        max: 60_000.0 / shortest,
    }
}

fn ruleset(mode: GameMode) -> Ruleset {
    match mode {
        GameMode::Osu => Ruleset::Osu,
        GameMode::Taiko => Ruleset::Taiko,
        GameMode::Catch => Ruleset::Fruits,
        GameMode::Mania => Ruleset::Mania,
    }
}

fn game_mode(ruleset: Ruleset) -> GameMode {
    match ruleset {
        Ruleset::Osu => GameMode::Osu,
        Ruleset::Taiko => GameMode::Taiko,
        Ruleset::Fruits => GameMode::Catch,
        Ruleset::Mania => GameMode::Mania,
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    /// osu!standard map: 120 BPM for 10 s, then 240 BPM for 2 s.
    const STD_MAP: &str = "osu file format v14

[General]
Mode: 0

[Difficulty]
HPDrainRate:5
CircleSize:4
OverallDifficulty:8
ApproachRate:9
SliderMultiplier:1.4
SliderTickRate:1

[TimingPoints]
0,500,4,2,0,100,1,0
10000,250,4,2,0,100,1,0

[HitObjects]
256,192,0,1,0,0:0:0:0:
128,192,2500,1,0,0:0:0:0:
384,192,5000,1,0,0:0:0:0:
256,100,7500,1,0,0:0:0:0:
256,300,10000,1,0,0:0:0:0:
128,192,12000,1,0,0:0:0:0:
";

    fn paths_for(osu: &Path) -> BeatmapPaths {
        BeatmapPaths {
            game: PathBuf::new(),
            songs: PathBuf::new(),
            folder: PathBuf::new(),
            osu: osu.to_path_buf(),
            background: None,
            audio: None,
        }
    }

    fn write_map(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("osu-memory-reader-{}-{name}.osu", std::process::id()));
        std::fs::write(&path, STD_MAP).unwrap();
        path
    }

    #[test]
    fn native_mode() {
        let path = write_map("native");
        let attrs = attributes(&paths_for(&path), None).unwrap();
        std::fs::remove_file(&path).unwrap();

        assert_eq!(attrs.original_mode, Ruleset::Osu);
        assert_eq!(attrs.mode, Ruleset::Osu);
        assert_eq!(attrs.key_count, None);
        assert!(attrs.stars > 0.0);
        assert_eq!(attrs.bpm, Bpm { common: 120.0, min: 120.0, max: 240.0 });
    }

    #[test]
    fn converted_to_mania() {
        let path = write_map("mania");
        let attrs = attributes(&paths_for(&path), Some(Ruleset::Mania)).unwrap();
        std::fs::remove_file(&path).unwrap();

        assert_eq!(attrs.original_mode, Ruleset::Osu);
        assert_eq!(attrs.mode, Ruleset::Mania);
        assert!(attrs.key_count.is_some_and(|k| (1..=10).contains(&k)));
    }

    #[test]
    fn missing_file() {
        let err = attributes(&paths_for(Path::new("/nonexistent/map.osu")), None).unwrap_err();
        assert!(matches!(err, Error::Parse { .. }));
    }
}
