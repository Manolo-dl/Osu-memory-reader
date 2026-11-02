// SPDX-License-Identifier: LGPL-3.0-only
// Copyright (C) 2026 Manolo-dl
//
// Based on tosu v4.26.3 (https://github.com/tosuapp/tosu):
//   packages/tosu/src/memory/stable.ts
//   packages/tosu/src/utils/bindings.ts
//   packages/common/enums/osu.ts
// Copyright (C) 2023-2026 Mikhail Babynichev and contributors.
// Modified in November 2026: ported from TypeScript to Rust.

use std::fmt;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::macros::raw_enum;
use crate::process::Process;
use crate::stable::{Addresses, Sig};
use crate::Result;

raw_enum! {
    /// Release stream the game updates from.
    pub enum ReleaseStream {
        /// Cutting edge.
        CuttingEdge = 0 => "cuttingEdge",
        /// Stable.
        Stable = 1 => "stable",
        /// Beta.
        Beta = 2 => "beta",
        /// Fallback.
        Fallback = 3 => "fallback",
    }
}

raw_enum! {
    /// Hit error meter shown during gameplay.
    pub enum ScoreMeterType {
        /// None.
        None = 0 => "none",
        /// Colour bar.
        Colour = 1 => "colour",
        /// Hit error bar.
        Error = 2 => "error",
    }
}

raw_enum! {
    /// Song progress indicator shown during gameplay.
    pub enum ProgressBarType {
        /// Off.
        Off = 0 => "off",
        /// Pie.
        Pie = 1 => "pie",
        /// Bar at the top right.
        TopRight = 2 => "topRight",
        /// Bar at the bottom right.
        BottomRight = 3 => "bottomRight",
        /// Bar at the bottom.
        Bottom = 4 => "bottom",
    }
}

raw_enum! {
    /// Leaderboard selected in song select.
    pub enum LeaderboardType {
        /// Local scores.
        Local = 0 => "local",
        /// Global ranking.
        Global = 1 => "global",
        /// Global ranking with the selected mods.
        SelectedMods = 2 => "selectedmods",
        /// Friends.
        Friends = 3 => "friends",
        /// Country.
        Country = 4 => "country",
    }
}

raw_enum! {
    /// Grouping of song select.
    pub enum GroupType {
        /// No grouping.
        None = 0 => "none",
        /// By artist.
        Artist = 1 => "artist",
        /// By BPM.
        Bpm = 2 => "bPM",
        /// By mapper.
        Creator = 3 => "creator",
        /// By date added.
        Date = 4 => "date",
        /// By difficulty.
        Difficulty = 5 => "difficulty",
        /// By length.
        Length = 6 => "length",
        /// By grade achieved.
        Rank = 7 => "rank",
        /// The user's own maps.
        MyMaps = 8 => "myMaps",
        /// Search results ("show all").
        Search = 12 => "search",
        /// By title.
        Title = 13 => "title",
        /// By last played.
        LastPlayed = 14 => "lastPlayed",
        /// Online favourites.
        OnlineFavourites = 15 => "onlineFavourites",
        /// By mania key count.
        ManiaKeys = 16 => "maniaKeys",
        /// By mode.
        Mode = 17 => "mode",
        /// By collection.
        Collection = 18 => "collection",
        /// By ranked status.
        RankedStatus = 19 => "rankedStatus",
    }
}

raw_enum! {
    /// Sorting of song select.
    pub enum SortType {
        /// By artist.
        Artist = 0 => "artist",
        /// By BPM.
        Bpm = 1 => "bpm",
        /// By mapper.
        Creator = 2 => "creator",
        /// By date added.
        Date = 3 => "date",
        /// By difficulty.
        Difficulty = 4 => "difficulty",
        /// By length.
        Length = 5 => "length",
        /// By grade achieved.
        Rank = 6 => "rank",
        /// By title.
        Title = 7 => "title",
    }
}

/// A keyboard key, as a Windows virtual-key code.
///
/// With the `serde` feature it is (de)serialized as
/// `{ "code": 90, "name": "Z" }`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct VirtualKey(pub u32);

impl VirtualKey {
    /// Name of the key as tosu shows it (`Z`, `LeftShift`, `D1`...), or
    /// `None` for codes it doesn't know.
    pub fn name(self) -> Option<&'static str> {
        Some(match self.0 {
            0 => "None",
            8 => "Back",
            9 => "Tab",
            13 => "Enter",
            20 => "CapsLock",
            27 => "Escape",
            32 => "Space",
            33 => "PageUp",
            34 => "PageDown",
            35 => "End",
            36 => "Home",
            37 => "Left",
            38 => "Up",
            39 => "Right",
            40 => "Down",
            41 => "Select",
            42 => "Print",
            43 => "Execute",
            44 => "PrintScreen",
            45 => "Insert",
            46 => "Delete",
            47 => "Help",
            48 => "D0",
            49 => "D1",
            50 => "D2",
            51 => "D3",
            52 => "D4",
            53 => "D5",
            54 => "D6",
            55 => "D7",
            56 => "D8",
            57 => "D9",
            65 => "A",
            66 => "B",
            67 => "C",
            68 => "D",
            69 => "E",
            70 => "F",
            71 => "G",
            72 => "H",
            73 => "I",
            74 => "J",
            75 => "K",
            76 => "L",
            77 => "M",
            78 => "N",
            79 => "O",
            80 => "P",
            81 => "Q",
            82 => "R",
            83 => "S",
            84 => "T",
            85 => "U",
            86 => "V",
            87 => "W",
            88 => "X",
            89 => "Y",
            90 => "Z",
            91 => "LeftWindows",
            92 => "RightWindows",
            93 => "Apps",
            95 => "Sleep",
            96 => "NumPad0",
            97 => "NumPad1",
            98 => "NumPad2",
            99 => "NumPad3",
            100 => "NumPad4",
            101 => "NumPad5",
            102 => "NumPad6",
            103 => "NumPad7",
            104 => "NumPad8",
            105 => "NumPad9",
            106 => "Multiply",
            107 => "Add",
            108 => "Separator",
            109 => "Subtract",
            110 => ".",
            111 => "/",
            112 => "F1",
            113 => "F2",
            114 => "F3",
            115 => "F4",
            116 => "F5",
            117 => "F6",
            118 => "F7",
            119 => "F8",
            120 => "F9",
            121 => "F10",
            122 => "F11",
            123 => "F12",
            124 => "F13",
            125 => "F14",
            126 => "F15",
            127 => "F16",
            128 => "F17",
            129 => "F18",
            130 => "F19",
            131 => "F20",
            132 => "F21",
            133 => "F22",
            134 => "F23",
            135 => "F24",
            144 => "NumLock",
            145 => "Scroll",
            160 => "LeftShift",
            161 => "RightShift",
            162 => "LeftControl",
            163 => "RightControl",
            164 => "LeftAlt",
            165 => "RightAlt",
            166 => "BrowserBack",
            167 => "BrowserForward",
            168 => "BrowserRefresh",
            169 => "BrowserStop",
            170 => "BrowserSearch",
            171 => "BrowserFavorites",
            172 => "BrowserHome",
            173 => "VolumeMute",
            174 => "VolumeDown",
            175 => "VolumeUp",
            176 => "MediaNextTrack",
            177 => "MediaPreviousTrack",
            178 => "MediaStop",
            179 => "MediaPlayPause",
            180 => "LaunchMail",
            181 => "SelectMedia",
            182 => "LaunchApplication1",
            183 => "LaunchApplication2",
            186 => ";",
            187 => "+",
            188 => ",",
            189 => "-",
            190 => ".",
            191 => "?",
            192 => "~",
            219 => "[",
            220 => "|",
            221 => "]",
            222 => "\"",
            223 => "Oem8",
            226 => "\\",
            229 => "ProcessKey",
            246 => "Attn",
            247 => "Crsel",
            248 => "Exsel",
            249 => "EraseEof",
            250 => "Play",
            251 => "Zoom",
            253 => "Pa1",
            254 => "OemClear",
            _ => return None,
        })
    }
}

/// The key name, or the code in hex if the name is unknown.
impl fmt::Display for VirtualKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.name() {
            Some(name) => f.write_str(name),
            None => write!(f, "{:#04x}", self.0),
        }
    }
}

#[cfg(feature = "serde")]
mod serde_impl {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    use super::VirtualKey;

    #[derive(Serialize)]
    struct KeyOut {
        code: u32,
        name: String,
    }

    #[derive(Deserialize)]
    struct KeyIn {
        code: u32,
    }

    impl Serialize for VirtualKey {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            KeyOut { code: self.0, name: self.to_string() }.serialize(serializer)
        }
    }

    impl<'de> Deserialize<'de> for VirtualKey {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            KeyIn::deserialize(deserializer).map(|k| VirtualKey(k.code))
        }
    }
}

/// Key bindings. Mania keys are configured per key count and are not here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(rename_all = "camelCase"))]
#[non_exhaustive]
pub struct Keybinds {
    /// osu!standard left click (K1).
    pub osu_left: VirtualKey,
    /// osu!standard right click (K2).
    pub osu_right: VirtualKey,
    /// osu!standard smoke.
    pub osu_smoke: VirtualKey,
    /// Taiko inner left (don).
    pub taiko_inner_left: VirtualKey,
    /// Taiko inner right (don).
    pub taiko_inner_right: VirtualKey,
    /// Taiko outer left (kat).
    pub taiko_outer_left: VirtualKey,
    /// Taiko outer right (kat).
    pub taiko_outer_right: VirtualKey,
    /// Catch left.
    pub fruits_left: VirtualKey,
    /// Catch right.
    pub fruits_right: VirtualKey,
    /// Catch dash.
    pub fruits_dash: VirtualKey,
    /// Mania: increase scroll speed.
    pub increase_speed: VirtualKey,
    /// Mania: decrease scroll speed.
    pub decrease_speed: VirtualKey,
    /// Pause.
    pub pause: VirtualKey,
    /// Skip the intro.
    pub skip: VirtualKey,
    /// Show or hide the leaderboard.
    pub toggle_scoreboard: VirtualKey,
    /// Increase the local offset.
    pub increase_audio_offset: VirtualKey,
    /// Decrease the local offset.
    pub decrease_audio_offset: VirtualKey,
    /// Quick retry.
    pub quick_retry: VirtualKey,
}

/// Audio settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(rename_all = "camelCase"))]
#[non_exhaustive]
pub struct AudioSettings {
    /// Master volume, 0 to 100.
    pub master_volume: i32,
    /// Music volume, 0 to 100.
    pub music_volume: i32,
    /// Effects volume, 0 to 100.
    pub effect_volume: i32,
    /// Universal offset in milliseconds.
    pub offset: i32,
    /// Ignore the beatmap's hitsounds.
    pub ignore_beatmap_sounds: bool,
    /// Use the skin's samples.
    pub use_skin_samples: bool,
}

/// Game client information.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(rename_all = "camelCase"))]
#[non_exhaustive]
pub struct ClientSettings {
    /// Game version, e.g. `20250107.1`.
    pub version: String,
    /// Release stream.
    pub branch: ReleaseStream,
    /// Whether an update is waiting to be installed.
    pub update_available: bool,
}

/// Background settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(rename_all = "camelCase"))]
#[non_exhaustive]
pub struct BackgroundSettings {
    /// Background dim, 0 to 100.
    pub dim: i32,
    /// Show storyboards.
    pub storyboard: bool,
}

/// Cursor settings.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(rename_all = "camelCase"))]
#[non_exhaustive]
pub struct CursorSettings {
    /// Cursor size multiplier.
    pub size: f64,
    /// Shrink the cursor on small circle sizes.
    pub auto_size: bool,
    /// Use the skin's cursor.
    pub use_skin_cursor: bool,
}

/// Mouse settings.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(rename_all = "camelCase"))]
#[non_exhaustive]
pub struct MouseSettings {
    /// Sensitivity multiplier.
    pub sensitivity: f64,
    /// Raw input.
    pub raw_input: bool,
    /// Disable mouse buttons during play.
    pub disable_buttons: bool,
    /// Disable the mouse wheel during play.
    pub disable_wheel: bool,
}

/// Window resolution settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(rename_all = "camelCase"))]
#[non_exhaustive]
pub struct ResolutionSettings {
    /// Fullscreen mode.
    pub fullscreen: bool,
    /// Windowed width.
    pub width: i32,
    /// Windowed height.
    pub height: i32,
    /// Fullscreen width.
    pub width_fullscreen: i32,
    /// Fullscreen height.
    pub height_fullscreen: i32,
}

/// osu!mania settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(rename_all = "camelCase"))]
#[non_exhaustive]
pub struct ManiaSettings {
    /// Global scroll speed. See also
    /// [`OsuReader::mania_scroll_speed`](crate::OsuReader::mania_scroll_speed)
    /// for the one in use with per-beatmap speeds.
    pub scroll_speed: i32,
    /// Scale the scroll speed with the BPM.
    pub speed_bpm_scale: bool,
    /// Remember the scroll speed per beatmap.
    pub use_per_beatmap_speed_scale: bool,
}

/// Skin settings.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(rename_all = "camelCase"))]
#[non_exhaustive]
pub struct SkinSettings {
    /// Name of the selected skin.
    pub name: String,
    /// Use the default skin in the editor.
    pub use_default_skin_in_editor: bool,
    /// Tint the slider ball with the combo colour.
    pub tint_slider_ball: bool,
    /// Ignore the beatmap's skin.
    pub ignore_beatmap_skins: bool,
    /// Use the skin's taiko elements.
    pub use_taiko_skin: bool,
}

/// Hit error meter settings.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(rename_all = "camelCase"))]
#[non_exhaustive]
pub struct ScoreMeterSettings {
    /// Meter shown.
    pub kind: ScoreMeterType,
    /// Size multiplier.
    pub size: f64,
}

/// Game settings, the same subset tosu exposes. Settings that couldn't be
/// read keep their default value.
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(rename_all = "camelCase"))]
#[non_exhaustive]
pub struct Settings {
    /// Audio.
    pub audio: AudioSettings,
    /// Game client.
    pub client: ClientSettings,
    /// Background.
    pub background: BackgroundSettings,
    /// Cursor.
    pub cursor: CursorSettings,
    /// Mouse.
    pub mouse: MouseSettings,
    /// Window resolution.
    pub resolution: ResolutionSettings,
    /// osu!mania.
    pub mania: ManiaSettings,
    /// Skin.
    pub skin: SkinSettings,
    /// Hit error meter.
    pub score_meter: ScoreMeterSettings,
    /// Song progress indicator.
    pub progress_bar: ProgressBarType,
    /// Leaderboard selected in song select.
    pub leaderboard: LeaderboardType,
    /// Song select grouping.
    pub group: GroupType,
    /// Song select sorting.
    pub sort: SortType,
    /// Key bindings.
    pub keybinds: Keybinds,
}

/// More than this many entries means the dictionary pointer is stale.
const MAX_ENTRIES: i32 = 4096;

/// Entries of a .NET `Dictionary` as (key, value) words: entries array at
/// +0x8, count at +0x1C; each entry is 16 bytes with the key at +0x0.
fn dictionary_entries(p: &Process, dictionary: u64) -> Result<Vec<[u8; 16]>> {
    let entries = p.read_ptr(dictionary + 0x8)?;
    let count = p.read_i32(dictionary + 0x1c)?;
    if entries == 0 || !(0..=MAX_ENTRIES).contains(&count) {
        return Ok(Vec::new());
    }
    let mut buf = vec![0u8; count as usize * 16];
    p.read_into(entries + 0x8, &mut buf)?;
    Ok(buf.as_chunks::<16>().0.to_vec())
}

fn word(entry: &[u8; 16], offset: usize) -> u32 {
    u32::from_le_bytes([entry[offset], entry[offset + 1], entry[offset + 2], entry[offset + 3]])
}

/// Bindable values: numbers are doubles at +0x4, flags and enums at +0xC,
/// strings are pointed to from +0x4.
struct Bindable<'a> {
    p: &'a Process,
    address: u64,
}

impl Bindable<'_> {
    fn int(&self) -> Result<i32> {
        Ok(self.double()?.round() as i32)
    }

    fn double(&self) -> Result<f64> {
        self.p.read_f64(self.address + 0x4)
    }

    fn bool(&self) -> Result<bool> {
        self.p.read_bool(self.address + 0xc)
    }

    fn raw(&self) -> Result<i32> {
        self.p.read_i32(self.address + 0xc)
    }

    fn string(&self) -> Result<String> {
        self.p.read_cs_string_ptr(self.address + 0x4)
    }
}

pub(crate) fn read(p: &Process, a: &Addresses) -> Result<Settings> {
    let mut s = Settings::default();

    let config = p.read_ptr_ptr(a.get(p, Sig::Configuration)?)?;
    for entry in dictionary_entries(p, config)? {
        let (key, value) = (u64::from(word(&entry, 0)), u64::from(word(&entry, 4)));
        if key == 0 || value == 0 {
            continue;
        }
        let b = Bindable { p, address: value };
        match p.read_cs_string(key)?.as_str() {
            "VolumeUniversal" => s.audio.master_volume = b.int()?,
            "VolumeMusic" => s.audio.music_volume = b.int()?,
            "VolumeEffect" => s.audio.effect_volume = b.int()?,
            "Offset" => s.audio.offset = b.int()?,
            "IgnoreBeatmapSamples" => s.audio.ignore_beatmap_sounds = b.bool()?,
            "SkinSamples" => s.audio.use_skin_samples = b.bool()?,
            "LastVersion" => s.client.version = b.string()?,
            "_ReleaseStream" => s.client.branch = ReleaseStream::from_raw(b.raw()?),
            "UpdatePending" => s.client.update_available = b.bool()?,
            "DimLevel" => s.background.dim = b.int()?,
            "ShowStoryboard" => s.background.storyboard = b.bool()?,
            "CursorSize" => s.cursor.size = b.double()?,
            "AutomaticCursorSizing" => s.cursor.auto_size = b.bool()?,
            "UseSkinCursor" => s.cursor.use_skin_cursor = b.bool()?,
            "MouseSpeed" => s.mouse.sensitivity = b.double()?,
            "RawInput" => s.mouse.raw_input = b.bool()?,
            "MouseDisableButtons" => s.mouse.disable_buttons = b.bool()?,
            "MouseDisableWheel" => s.mouse.disable_wheel = b.bool()?,
            "Fullscreen" => s.resolution.fullscreen = b.bool()?,
            "Width" => s.resolution.width = b.int()?,
            "Height" => s.resolution.height = b.int()?,
            "WidthFullscreen" => s.resolution.width_fullscreen = b.int()?,
            "HeightFullscreen" => s.resolution.height_fullscreen = b.int()?,
            "ManiaSpeed" => s.mania.scroll_speed = b.int()?,
            "ManiaSpeedBPMScale" => s.mania.speed_bpm_scale = b.bool()?,
            "UsePerBeatmapManiaSpeed" => s.mania.use_per_beatmap_speed_scale = b.bool()?,
            "Skin" => s.skin.name = b.string()?,
            "EditorDefaultSkin" => s.skin.use_default_skin_in_editor = b.bool()?,
            "ComboColourSliderBall" => s.skin.tint_slider_ball = b.bool()?,
            "IgnoreBeatmapSkins" => s.skin.ignore_beatmap_skins = b.bool()?,
            "UseTaikoSkin" => s.skin.use_taiko_skin = b.bool()?,
            "ScoreMeter" => s.score_meter.kind = ScoreMeterType::from_raw(b.raw()?),
            "ScoreMeterScale" => s.score_meter.size = b.double()?,
            "ProgressBarType" => s.progress_bar = ProgressBarType::from_raw(b.raw()?),
            "RankType" => s.leaderboard = LeaderboardType::from_raw(b.raw()?),
            "TreeSortMode" => s.group = GroupType::from_raw(b.raw()?),
            "TreeSortMode2" => s.sort = SortType::from_raw(b.raw()?),
            _ => {}
        }
    }

    let bindings = p.read_ptr_ptr(a.get(p, Sig::Bindings)?)?;
    let k = &mut s.keybinds;
    for entry in dictionary_entries(p, bindings)? {
        // Key: binding id at +0x0; value: virtual-key code at +0xC.
        let key = VirtualKey(word(&entry, 0xc));
        let slot = match word(&entry, 0) {
            2 => &mut k.osu_left,
            3 => &mut k.osu_right,
            4 => &mut k.osu_smoke,
            6 => &mut k.taiko_inner_left,
            7 => &mut k.taiko_inner_right,
            8 => &mut k.taiko_outer_left,
            9 => &mut k.taiko_outer_right,
            11 => &mut k.fruits_left,
            12 => &mut k.fruits_right,
            13 => &mut k.fruits_dash,
            15 => &mut k.increase_speed,
            16 => &mut k.decrease_speed,
            18 => &mut k.pause,
            19 => &mut k.skip,
            20 => &mut k.toggle_scoreboard,
            21 => &mut k.increase_audio_offset,
            22 => &mut k.decrease_audio_offset,
            23 => &mut k.quick_retry,
            _ => continue,
        };
        *slot = key;
    }

    Ok(s)
}

/// Scroll speed in use, including per-beatmap speeds.
pub(crate) fn mania_scroll_speed(p: &Process, a: &Addresses) -> Result<i32> {
    let address = a.get(p, Sig::ManiaSpeed)?;
    p.read_i32(p.read_ptr(address + 0x1)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_names() {
        assert_eq!(VirtualKey(90).name(), Some("Z"));
        assert_eq!(VirtualKey(49).name(), Some("D1"));
        assert_eq!(VirtualKey(160).name(), Some("LeftShift"));
        assert_eq!(VirtualKey(7).name(), None);
        assert_eq!(VirtualKey(7).to_string(), "0x07");
    }
}
