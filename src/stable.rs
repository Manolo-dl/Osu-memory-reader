//! osu! stable reader. Offsets and patterns ported from tosu
//! (`packages/tosu/src/memory/stable.ts`).

use crate::pattern::Pattern;
use crate::process::{Process, Result};

const BASE_PATTERN: &str = "F8 01 74 04 83 65";

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
    pub checksum: String,
}

pub struct StableReader {
    pub process: Process,
    base: u64,
}

impl StableReader {
    /// Scans memory for the base pattern. Returns `None` if it isn't found
    /// (osu! may still be loading, or permissions are missing).
    pub fn attach(process: Process) -> Option<Self> {
        let base = process.scan_batch(&[Pattern::parse(BASE_PATTERN)])[0]?;
        Some(Self { process, base })
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
