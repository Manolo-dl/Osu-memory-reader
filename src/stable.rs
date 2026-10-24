//! osu! stable memory layout. Offsets and patterns ported from tosu
//! (`packages/tosu/src/memory/stable.ts`).

use crate::pattern::Pattern;
use crate::process::Process;
use crate::{Beatmap, Error, RankedStatus, Result, Ruleset};

/// (name, signature)
const PATTERNS: [(&str, &str); 2] = [("base", "F8 01 74 04 83 65"), ("settingsClass", "83 E0 20 85 C0 7E 2F")];

/// Addresses resolved from the signatures.
pub(crate) struct Addresses {
    base: u64,
    settings_class: u64,
}

impl Addresses {
    pub(crate) fn scan(process: &Process) -> Result<Self> {
        let patterns = PATTERNS.map(|(_, signature)| Pattern::parse(signature));
        let found = process.scan_batch(&patterns)?;

        let get = |i: usize| found[i].ok_or(Error::PatternNotFound { name: PATTERNS[i].0 });
        Ok(Self { base: get(0)?, settings_class: get(1)? })
    }
}

/// Songs folder exactly as written in the game settings (`BeatmapDirectory`).
pub(crate) fn songs_folder(p: &Process, addr: &Addresses) -> Result<String> {
    // [[[SettingsClass + 0x8] + 0xB8] + 0x4]
    let settings = p.read_ptr(addr.settings_class + 0x8)?;
    p.read_cs_string_ptr(p.read_ptr(settings + 0xb8)? + 0x4)
}

pub(crate) fn beatmap(p: &Process, addr: &Addresses) -> Result<Option<Beatmap>> {
    let base = addr.base;

    let bm = p.read_ptr_ptr(base - 0xc)?;
    if bm == 0 {
        return Ok(None);
    }

    Ok(Some(Beatmap {
        mode: Ruleset::from_id(p.read_ptr_ptr(base - 0x33)? as i32),
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
        ranked_status: RankedStatus::from_raw(p.read_i32(bm + 0x12c)?),
    }))
}
