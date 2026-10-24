// SPDX-License-Identifier: LGPL-3.0-only
// Copyright (C) 2026 Manolo-dl

//! Prints the selected beatmap and its paths every time it changes.
//!
//! ```sh
//! cargo run --release --example watch
//! cargo run --release --example watch -- --once
//! cargo run --release --example watch --features difficulty  # also stars, BPM, keys
//! ```

use std::path::Path;
use std::thread::sleep;
use std::time::Duration;

use osu_memory_reader::{Beatmap, BeatmapPaths, Error, OsuReader};

const POLL_INTERVAL: Duration = Duration::from_millis(250);
const RETRY_INTERVAL: Duration = Duration::from_secs(2);

fn main() {
    let once = std::env::args().any(|a| a == "--once");

    loop {
        let reader = attach();
        watch(&reader, once);
        if once {
            return;
        }
        eprintln!("osu! closed, waiting for it to start again...");
    }
}

/// Retries until osu! is running and its memory has been scanned.
fn attach() -> OsuReader {
    loop {
        match OsuReader::attach() {
            Ok(reader) => {
                eprintln!("attached to osu! (pid {})", reader.pid());
                return reader;
            }
            Err(Error::ProcessNotFound) => eprintln!("waiting for osu!..."),
            Err(e) => eprintln!("{e}"),
        }
        sleep(RETRY_INTERVAL);
    }
}

fn watch(reader: &OsuReader, once: bool) {
    let mut last_checksum = String::new();

    while reader.is_running() {
        match reader.beatmap() {
            Ok(Some(beatmap)) if beatmap.checksum != last_checksum => {
                print_beatmap(&beatmap);
                match reader.beatmap_paths(&beatmap) {
                    Ok(paths) => {
                        print_paths(&paths);
                        #[cfg(feature = "difficulty")]
                        print_attributes(&paths, &beatmap);
                    }
                    Err(e) => eprintln!("could not resolve paths: {e}"),
                }
                println!();
                last_checksum = beatmap.checksum;
            }
            Ok(_) => {}
            Err(e) => eprintln!("{e}"),
        }

        if once {
            return;
        }
        sleep(POLL_INTERVAL);
    }
}

fn print_beatmap(b: &Beatmap) {
    let mode = b.mode.map_or("?", |m| m.name());
    println!("{} - {} [{}] by {}", b.artist, b.title, b.difficulty, b.creator);
    println!("  AR {:.1}  CS {:.1}  HP {:.1}  OD {:.1}  objects {}", b.ar, b.cs, b.hp, b.od, b.object_count);
    println!("  mode {mode}  {}  id {} / set {}", b.ranked_status, b.map_id, b.set_id);
    println!("  md5 {}", b.checksum);
}

#[cfg(feature = "difficulty")]
fn print_attributes(paths: &BeatmapPaths, beatmap: &Beatmap) {
    match osu_memory_reader::attributes(paths, beatmap.mode) {
        Ok(a) => {
            let keys = a.key_count.map_or(String::new(), |k| format!("  {k}K"));
            println!("  {:.2}*  original mode {} -> {}{keys}", a.stars, a.original_mode, a.mode);
            println!("  BPM {:.0} ({:.0}-{:.0})", a.bpm.common, a.bpm.min, a.bpm.max);
        }
        Err(e) => eprintln!("could not compute attributes: {e}"),
    }
}

fn print_paths(p: &BeatmapPaths) {
    print_path("game", Some(&p.game));
    print_path("songs", Some(&p.songs));
    print_path("folder", Some(&p.folder));
    print_path(".osu", Some(&p.osu));
    print_path("bg", p.background.as_deref());
    print_path("audio", p.audio.as_deref());
}

/// Marks paths that don't exist on disk, to spot detection problems.
fn print_path(label: &str, path: Option<&Path>) {
    match path {
        Some(path) if path.exists() => println!("  {label:<8}{}", path.display()),
        Some(path) => println!("  {label:<8}{} (missing)", path.display()),
        None => println!("  {label:<8}-"),
    }
}
