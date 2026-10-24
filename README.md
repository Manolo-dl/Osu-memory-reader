# osu-memory-reader

Crate de Rust que lee de la memoria de **osu! stable** el beatmap seleccionado y las rutas de sus archivos en disco (`.osu`, carpeta, bg y audio).

Es un port del lector de stable de [tosu](https://github.com/tosuapp/tosu), sin servidor: devuelve valores de Rust y tú decides qué hacer con ellos.

Funciona en Windows y en Linux (osu! bajo Wine, por ejemplo con osu-winello).

## Uso

```toml
[dependencies]
osu-memory-reader = { git = "https://github.com/<usuario>/osu-memory-reader" }
# o, desarrollando ambos proyectos a la vez:
# osu-memory-reader = { path = "../osu-memory-reader" }
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

El lector no corre nada en segundo plano: llama a `beatmap()` cada 100–500 ms y compara `checksum` para saber cuándo cambia el mapa. `examples/watch.rs` tiene un bucle completo que además espera a que osu! se abra y se reengancha si se cierra:

```sh
cargo run --release --example watch
cargo run --release --example watch -- --once
```

La documentación completa de la API se genera con `cargo doc --open --all-features`.

## Features

- `serde`: `Serialize`/`Deserialize` en `Beatmap`, `BeatmapPaths`, `Ruleset` y `RankedStatus`, con los campos en camelCase (`rankedStatus`, `backgroundFilename`...).

## Ejemplo con Tauri

Con la feature `serde`, un hilo que avisa al frontend cada vez que cambia el mapa:

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
                // osu! no está abierto o todavía está cargando.
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

En el frontend: `listen("beatmap-changed", (e) => console.log(e.payload.beatmap.title))`.

## Rutas

- **Carpeta del juego:** se detecta sola. En Windows sale de la ruta de `osu!.exe`; en Linux, del directorio de trabajo del proceso o del `osupath` de osu-winello. Se puede forzar con `OsuReader::set_game_folder` o con la variable `OSU_PATH`.
- **Carpeta Songs:** se lee de los ajustes de osu! (`BeatmapDirectory`). Si es relativa, se une a la carpeta del juego. Bajo Wine, las rutas tipo `D:\Songs` se traducen a la ruta real con los `dosdevices` del prefix.

## Permisos

- **Windows:** si osu! corre como administrador, el programa que lee también tiene que hacerlo.
- **Linux:** según `ptrace_scope` del kernel puede hacer falta `sudo` o `sudo setcap cap_sys_ptrace=eip <binario>`.

## Licencia

LGPL-3.0, igual que tosu, del que salen los offsets y patrones de memoria.
