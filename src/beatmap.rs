use std::fmt;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Game mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(rename_all = "lowercase"))]
pub enum Ruleset {
    /// osu!standard.
    Osu = 0,
    /// osu!taiko.
    Taiko = 1,
    /// osu!catch.
    Fruits = 2,
    /// osu!mania.
    Mania = 3,
}

impl Ruleset {
    /// Converts the numeric id used by the game (`0..=3`).
    pub fn from_id(id: i32) -> Option<Self> {
        match id {
            0 => Some(Self::Osu),
            1 => Some(Self::Taiko),
            2 => Some(Self::Fruits),
            3 => Some(Self::Mania),
            _ => None,
        }
    }

    /// Numeric id used by the game.
    pub fn id(self) -> i32 {
        self as i32
    }

    /// Short lowercase name: `osu`, `taiko`, `fruits` or `mania`.
    pub fn name(self) -> &'static str {
        match self {
            Self::Osu => "osu",
            Self::Taiko => "taiko",
            Self::Fruits => "fruits",
            Self::Mania => "mania",
        }
    }
}

impl fmt::Display for Ruleset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Online status of a beatmap, as shown by osu! stable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(rename_all = "camelCase"))]
pub enum RankedStatus {
    /// Not known yet (for example, the game hasn't checked online).
    #[default]
    Unknown,
    /// Not submitted to the website.
    NotSubmitted,
    /// Pending, WIP or graveyard.
    Pending,
    /// Ranked.
    Ranked,
    /// Approved.
    Approved,
    /// Qualified.
    Qualified,
    /// Loved.
    Loved,
}

impl RankedStatus {
    /// Converts the raw value stored by osu! stable. Unrecognised values map to
    /// [`RankedStatus::Unknown`].
    pub fn from_raw(value: i32) -> Self {
        match value {
            1 => Self::NotSubmitted,
            2 => Self::Pending,
            4 => Self::Ranked,
            5 => Self::Approved,
            6 => Self::Qualified,
            7 => Self::Loved,
            _ => Self::Unknown,
        }
    }

    /// camelCase name, e.g. `ranked` or `notSubmitted`.
    pub fn name(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::NotSubmitted => "notSubmitted",
            Self::Pending => "pending",
            Self::Ranked => "ranked",
            Self::Approved => "approved",
            Self::Qualified => "qualified",
            Self::Loved => "loved",
        }
    }
}

impl fmt::Display for RankedStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// The beatmap currently selected in the game.
///
/// File and folder names are as stored by the game; use
/// [`OsuReader::beatmap_paths`](crate::OsuReader::beatmap_paths) to get full
/// paths on disk.
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(rename_all = "camelCase"))]
#[non_exhaustive]
pub struct Beatmap {
    /// Ruleset currently selected in the game. For converted maps this is the
    /// target mode, not the map's original one.
    pub mode: Option<Ruleset>,
    /// Artist (romanised).
    pub artist: String,
    /// Title (romanised).
    pub title: String,
    /// Mapper.
    pub creator: String,
    /// Difficulty name.
    pub difficulty: String,
    /// Approach rate.
    pub ar: f32,
    /// Circle size (key count in mania).
    pub cs: f32,
    /// HP drain rate.
    pub hp: f32,
    /// Overall difficulty.
    pub od: f32,
    /// Online beatmap id (`0` if not submitted).
    pub map_id: i32,
    /// Online beatmap set id (`0` if not submitted).
    pub set_id: i32,
    /// Online status.
    pub ranked_status: RankedStatus,
    /// Number of hit objects.
    pub object_count: i32,
    /// Folder name inside the songs folder.
    pub folder: String,
    /// `.osu` file name.
    pub filename: String,
    /// Audio file name, relative to [`folder`](Self::folder).
    pub audio_filename: String,
    /// Background file name, relative to [`folder`](Self::folder). May be empty.
    pub background_filename: String,
    /// MD5 of the `.osu` file. Changes when the selected beatmap changes, so
    /// it is a cheap way to detect a new selection.
    pub checksum: String,
}
