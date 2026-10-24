// SPDX-License-Identifier: LGPL-3.0-only
// Copyright (C) 2026 Manolo-dl
//
// Based on tosu v4.26.3 (https://github.com/tosuapp/tosu):
//   packages/tosu/src/instances/osuInstance.ts
//   packages/tosu/src/states/beatmap.ts
// Copyright (C) 2023-2026 Mikhail Babynichev and contributors.
// Modified in October 2026: ported from TypeScript to Rust.

use std::path::PathBuf;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::Beatmap;
use crate::process::Process;

/// Full paths on disk of the selected beatmap and the folders around it.
///
/// Paths are built from what the game reports and are not checked; a file
/// can be missing if the beatmap was deleted from disk.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(rename_all = "camelCase"))]
#[non_exhaustive]
pub struct BeatmapPaths {
    /// osu! installation folder.
    pub game: PathBuf,
    /// Songs folder.
    pub songs: PathBuf,
    /// Folder of the beatmap set.
    pub folder: PathBuf,
    /// `.osu` file.
    pub osu: PathBuf,
    /// Background image, if the beatmap has one.
    pub background: Option<PathBuf>,
    /// Audio file, if the beatmap has one.
    pub audio: Option<PathBuf>,
}

/// Mirrors tosu: the songs folder comes from the game settings and is relative
/// to the game folder unless it is absolute. `raw_songs` empty means `Songs`.
pub(crate) fn resolve(process: &Process, game: PathBuf, raw_songs: &str, beatmap: &Beatmap) -> BeatmapPaths {
    let songs = match raw_songs.trim() {
        "" => game.join("Songs"),
        raw => {
            let path = process.host_path(raw);
            if path.is_absolute() { path } else { game.join(path) }
        }
    };

    let folder = songs.join(process.host_path(&beatmap.folder));
    let file_in_folder = |name: &str| (!name.is_empty()).then(|| folder.join(process.host_path(name)));

    BeatmapPaths {
        osu: folder.join(process.host_path(&beatmap.filename)),
        background: file_in_folder(&beatmap.background_filename),
        audio: file_in_folder(&beatmap.audio_filename),
        game,
        songs,
        folder,
    }
}
