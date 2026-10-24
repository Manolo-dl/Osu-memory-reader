//! Resolves the beatmap files on disk, the same way tosu does: songs folder
//! from the game settings, relative to the game folder unless it is absolute.

use std::env;
use std::path::PathBuf;

use crate::process::Process;
use crate::stable::Beatmap;

pub struct BeatmapPaths {
    pub game: PathBuf,
    pub songs: PathBuf,
    pub folder: PathBuf,
    pub osu: PathBuf,
    pub background: Option<PathBuf>,
    pub audio: Option<PathBuf>,
}

/// Folder of the game. `OSU_PATH` overrides the detection.
pub fn game_folder(process: &Process) -> Option<PathBuf> {
    env::var_os("OSU_PATH").map(PathBuf::from).or_else(|| process.game_folder())
}

/// `raw_songs` is the songs folder read from memory; empty means the default `Songs`.
pub fn resolve(process: &Process, game: PathBuf, raw_songs: &str, beatmap: &Beatmap) -> BeatmapPaths {
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
