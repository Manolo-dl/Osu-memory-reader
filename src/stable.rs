//! osu! stable reader. Offsets and patterns ported from tosu
//! (`packages/tosu/src/memory/stable.ts`).

use crate::pattern::Pattern;
use crate::process::{Process, Result};

const BASE_PATTERN: &str = "F8 01 74 04 83 65";
const SETTINGS_CLASS_PATTERN: &str = "83 E0 20 85 C0 7E 2F";

#[derive(Debug, Default)]
pub struct Beatmap {
    pub mode: i32,
    pub artist: String,
    pub title: String,
    pub creator: String,
    pub difficulty: String,
    pub ar: f32,
    pub cs: f32,
    pub hp: f32,
    pub od: f32,
    pub map_id: i32,
    pub set_id: i32,
    pub ranked_status: i32,
    pub object_count: i32,
    pub folder: String,
    pub filename: String,
    pub audio_filename: String,
    pub background_filename: String,
    pub checksum: String,
}

pub struct StableReader {
    pub process: Process,
    base: u64,
    settings_class: u64,
}

impl StableReader {
    /// Scans memory for the patterns. Returns `None` if any is missing
    /// (osu! may still be loading, or permissions are missing).
    pub fn attach(process: Process) -> Option<Self> {
        let patterns = [Pattern::parse(BASE_PATTERN), Pattern::parse(SETTINGS_CLASS_PATTERN)];
        let [base, settings_class] = process.scan_batch(&patterns)[..] else {
            unreachable!();
        };
        Some(Self { process, base: base?, settings_class: settings_class? })
    }

    /// Songs folder from the game settings, as written in `BeatmapDirectory`
    /// (relative to the game folder, or absolute).
    pub fn songs_folder(&self) -> Result<String> {
        // [[[SettingsClass + 0x8] + 0xB8] + 0x4]
        let p = &self.process;
        let settings = p.read_ptr(self.settings_class + 0x8)?;
        p.read_cs_string_ptr(p.read_ptr(settings + 0xb8)? + 0x4)
    }

    pub fn beatmap(&self) -> Result<Option<Beatmap>> {
        let p = &self.process;
        let base = self.base;

        let bm = p.read_ptr_ptr(base - 0xc)?;
        if bm == 0 {
            return Ok(None);
        }

        Ok(Some(Beatmap {
            mode: p.read_ptr_ptr(base - 0x33)? as i32,
            artist: p.read_cs_string_ptr(bm + 0x18)?,
            title: p.read_cs_string_ptr(bm + 0x24)?,
            ar: p.read_f32(bm + 0x2c)?,
            cs: p.read_f32(bm + 0x30)?,
            hp: p.read_f32(bm + 0x34)?,
            od: p.read_f32(bm + 0x38)?,
            audio_filename: p.read_cs_string_ptr(bm + 0x64)?,
            background_filename: p.read_cs_string_ptr(bm + 0x68)?,
            checksum: p.read_cs_string_ptr(bm + 0x6c)?,
            folder: p.read_cs_string_ptr(bm + 0x78)?,
            creator: p.read_cs_string_ptr(bm + 0x7c)?,
            filename: p.read_cs_string_ptr(bm + 0x90)?,
            difficulty: p.read_cs_string_ptr(bm + 0xac)?,
            map_id: p.read_i32(bm + 0xc8)?,
            set_id: p.read_i32(bm + 0xcc)?,
            object_count: p.read_i32(bm + 0xf8)?,
            ranked_status: p.read_i32(bm + 0x12c)?,
        }))
    }
}
