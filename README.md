# osu-memory-reader

[![CI](https://github.com/Manolo-dl/Osu-memory-reader/actions/workflows/ci.yml/badge.svg)](https://github.com/Manolo-dl/Osu-memory-reader/actions/workflows/ci.yml)

A Rust crate that reads the selected beatmap and the paths of its files on disk (`.osu`, folder, background and audio) from the memory of a running **osu! stable** process.

It is a port of the stable reader in [tosu](https://github.com/tosuapp/tosu), without the server: it returns plain Rust values and you decide what to do with them.

Works on Windows and on Linux (osu! under Wine, for example with osu-winello).

## Usage

```toml
[dependencies]
osu-memory-reader = { version = "0.1", features = ["serde", "difficulty"] }
```

```rust
use osu_memory_reader::OsuReader;

let reader = OsuReader::attach()?;

if let Some(beatmap) = reader.beatmap()? {
    println!("{} - {} [{}]", beatmap.artist, beatmap.title, beatmap.difficulty);

    let paths = reader.beatmap_paths(&beatmap)?;
    println!(".osu: {}", paths.osu.display());
}
```

The reader runs nothing in the background: call `beatmap()` every 100–500 ms and compare `checksum` to detect when the selection changes. `examples/watch.rs` has a complete loop that also waits for osu! to start and reattaches when it closes:

```sh
cargo run --release --example watch
cargo run --release --example watch -- --once
cargo run --release --example watch --features difficulty   # also stars, BPM and keys
```

Build the full API documentation with `cargo doc --open --all-features`.

## Features

- `serde`: `Serialize`/`Deserialize` on all public data types, with camelCase fields (`rankedStatus`, `backgroundFilename`, ...).
- `difficulty`: adds `attributes(&paths, mode)`, which parses the `.osu` file with [rosu-pp](https://crates.io/crates/rosu-pp) to get:
  - the star rating without mods, in the selected mode (converting osu!standard maps when needed);
  - the BPM: most common, minimum and maximum;
  - the original mode of the map (`beatmap.mode` is the mode selected in the game, not the map's);
  - the key count when played in mania, including converts, where it depends on CS, OD and the objects, not just CS.

  It parses the whole file, so call it when `checksum` changes, not on every poll. `attributes_from_path` does the same for any `.osu` file.

```rust
let attrs = osu_memory_reader::attributes(&paths, beatmap.mode)?;
println!("{:.2}*  {:.0} BPM  {:?} keys", attrs.stars, attrs.bpm.common, attrs.key_count);
```

## Tauri example

With the `serde` feature, a thread that notifies the frontend every time the beatmap changes:

```rust
use std::{thread, time::Duration};
use osu_memory_reader::{Beatmap, BeatmapPaths, OsuReader};
use tauri::Emitter;

#[derive(Clone, serde::Serialize)]
struct BeatmapChanged {
    beatmap: Beatmap,
    paths: Option<BeatmapPaths>,
}

fn spawn_reader(app: tauri::AppHandle) {
    thread::spawn(move || loop {
        let reader = match OsuReader::attach() {
            Ok(reader) => reader,
            Err(_) => {
                // osu! is not running or is still loading.
                thread::sleep(Duration::from_secs(2));
                continue;
            }
        };

        let mut last_checksum = String::new();
        while reader.is_running() {
            if let Ok(Some(beatmap)) = reader.beatmap() {
                if beatmap.checksum != last_checksum {
                    last_checksum = beatmap.checksum.clone();
                    let paths = reader.beatmap_paths(&beatmap).ok();
                    let _ = app.emit("beatmap-changed", BeatmapChanged { beatmap, paths });
                }
            }
            thread::sleep(Duration::from_millis(250));
        }
    });
}
```

In the frontend: `listen("beatmap-changed", (e) => console.log(e.payload.beatmap.title))`.

## Paths

- **Game folder:** detected automatically. On Windows it comes from the path of `osu!.exe`; on Linux, from the process working directory or osu-winello's `osupath`. It can be overridden with `OsuReader::set_game_folder` or the `OSU_PATH` environment variable.
- **Songs folder:** read from the osu! settings (`BeatmapDirectory`). If relative, it is joined to the game folder. Under Wine, paths like `D:\Songs` are translated to the real path through the prefix's `dosdevices`.

## Permissions

- **Windows:** if osu! runs as administrator, the reading program must too.
- **Linux:** depending on the kernel's `ptrace_scope`, it may need `sudo` or `sudo setcap cap_sys_ptrace=eip <binary>`.

## License

Copyright (C) 2026 Manolo-dl

Modified version of parts of [tosu](https://github.com/tosuapp/tosu) v4.26.3, Copyright (C) 2023-2026 Mikhail Babynichev and contributors, where the memory patterns and offsets come from. [NOTICE](NOTICE) lists where each file comes from and what was changed.

This crate is free software: you can redistribute it and/or modify it under the terms of the GNU Lesser General Public License version 3 (LGPL-3.0-only). See [COPYING.LESSER](COPYING.LESSER) and [COPYING](COPYING) (the GPL-3.0, which the LGPL builds on).

Using it as a dependency does not require changing your project's license, but when you distribute a binary that includes it you must ship the LGPL-3.0 text with it, state that it uses this crate, and allow it to be replaced (for example, by publishing your project's source code).
