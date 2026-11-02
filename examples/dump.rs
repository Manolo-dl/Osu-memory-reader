// SPDX-License-Identifier: LGPL-3.0-only
// Copyright (C) 2026 Manolo-dl

//! Calls every reader method once and prints the result and how long it took.
//!
//! ```sh
//! cargo run --release --example dump
//! ```

use std::fmt::Debug;
use std::time::Instant;

use osu_memory_reader::{BassDensity, OsuReader, unstable_rate};

fn show<T: Debug>(name: &str, read: impl FnOnce() -> osu_memory_reader::Result<T>) -> Option<T> {
    let start = Instant::now();
    let result = read();
    let elapsed = start.elapsed();
    match &result {
        Ok(value) => println!("{name} ({elapsed:.1?}): {value:#?}"),
        Err(e) => println!("{name} ({elapsed:.1?}): error: {e}"),
    }
    result.ok()
}

fn main() {
    let start = Instant::now();
    let reader = match OsuReader::attach() {
        Ok(reader) => reader,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };
    println!("attached to pid {} in {:.1?}\n", reader.pid(), start.elapsed());

    show("status", || reader.status());
    show("play_time", || reader.play_time());
    show("game_time", || reader.game_time());
    if let Some(mods) = show("menu_mods", || reader.menu_mods()) {
        println!("  -> {}", if mods.is_empty() { "NM".to_string() } else { mods.to_string() });
    }
    show("chat_status", || reader.chat_status());
    show("is_interface_visible", || reader.is_interface_visible());
    show("is_watching_replay", || reader.is_watching_replay());
    show("is_replay_ui_hidden", || reader.is_replay_ui_hidden());
    show("skin_folder", || reader.skin_folder());
    show("songs_folder", || reader.songs_folder());
    show("beatmap", || reader.beatmap());
    show("audio_length", || reader.audio_length());
    if let Some(Some(spectrum)) = show("audio_spectrum", || reader.audio_spectrum()) {
        println!("  -> bass density {:.2}", BassDensity::new().update(&spectrum));
    }
    show("user", || reader.user());

    if let Some(Some(play)) = show("gameplay", || reader.gameplay()) {
        if let Some(mode) = play.mode {
            println!(
                "  -> accuracy {:.2}%  grade {}",
                play.hits.accuracy(mode, play.mods),
                play.hits.grade(mode, play.mods)
            );
        }
        if let Ok(Some(errors)) = reader.hit_errors(0) {
            println!("  -> unstable rate {:.2}", unstable_rate(&errors, play.mods));
        }
    }
    show("key_overlay", || reader.key_overlay());
    show("hit_errors", || reader.hit_errors(0).map(|e| e.map(|e| e.len())));
    show("leaderboard", || reader.leaderboard());
    show("result_screen", || reader.result_screen());
    show("settings", || reader.settings());
    show("mania_scroll_speed", || reader.mania_scroll_speed());
    show("tourney", || reader.tourney());
    show("tourney_chat", || reader.tourney_chat());
    show("spectating_user", || reader.spectating_user());
}
