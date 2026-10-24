mod enums;
mod paths;
mod pattern;
mod process;
mod stable;

use std::path::PathBuf;
use std::thread::sleep;
use std::time::Duration;

use paths::BeatmapPaths;
use process::Process;
use stable::{Beatmap, StableReader};

const POLL_INTERVAL: Duration = Duration::from_millis(250);
const RETRY_INTERVAL: Duration = Duration::from_secs(2);

fn main() {
    let once = std::env::args().any(|a| a == "--once");

    loop {
        let reader = attach();
        run(&reader, once);
        if once {
            return;
        }
        eprintln!("osu! se cerró, esperando a que vuelva a abrirse...");
    }
}

/// Waits for an osu! stable process and for the base pattern to be found.
fn attach() -> StableReader {
    loop {
        let Some(&pid) = Process::find(&["osu!.exe"]).first() else {
            eprintln!("esperando a osu!.exe...");
            sleep(RETRY_INTERVAL);
            continue;
        };

        let process = match Process::open(pid) {
            Ok(process) => process,
            Err(e) => {
                eprintln!("no se pudo abrir osu! (pid {pid}): {e} ({})", process::PERMISSION_HINT);
                sleep(RETRY_INTERVAL);
                continue;
            }
        };

        eprintln!("osu! encontrado (pid {pid}), escaneando memoria...");
        match StableReader::attach(process) {
            Some(reader) => return reader,
            None => {
                eprintln!("patrón no encontrado (¿permisos? {})", process::PERMISSION_HINT);
                sleep(RETRY_INTERVAL);
            }
        }
    }
}

/// Prints the beatmap every time it changes (by its md5).
fn run(reader: &StableReader, once: bool) {
    let game = paths::game_folder(&reader.process);
    if game.is_none() {
        eprintln!("no se pudo encontrar la carpeta de osu! (usa OSU_PATH para indicarla)");
    }

    let mut last_checksum = String::new();

    while reader.process.is_alive() {
        match reader.beatmap() {
            Ok(Some(b)) if b.checksum != last_checksum => {
                print_beatmap(&b);
                if let Some(game) = &game {
                    match reader.songs_folder() {
                        Ok(songs) => print_paths(&paths::resolve(&reader.process, game.clone(), &songs, &b)),
                        Err(e) => eprintln!("error leyendo la carpeta Songs: {e}"),
                    }
                }
                println!();
                last_checksum = b.checksum;
            }
            Ok(_) => {}
            Err(e) => eprintln!("error leyendo beatmap: {e}"),
        }

        if once {
            return;
        }
        sleep(POLL_INTERVAL);
    }
}

fn print_beatmap(b: &Beatmap) {
    println!("{} - {} [{}] por {}", b.artist, b.title, b.difficulty, b.creator);
    println!("  AR {:.1}  CS {:.1}  HP {:.1}  OD {:.1}  objetos {}", b.ar, b.cs, b.hp, b.od, b.object_count);
    println!("  modo {}  {}  id {} / set {}", enums::ruleset(b.mode), enums::ranked_status(b.ranked_status), b.map_id, b.set_id);
    println!("  md5 {}", b.checksum);
}

fn print_paths(p: &BeatmapPaths) {
    print_path("juego", Some(&p.game));
    print_path("songs", Some(&p.songs));
    print_path("carpeta", Some(&p.folder));
    print_path(".osu", Some(&p.osu));
    print_path("bg", p.background.as_ref());
    print_path("audio", p.audio.as_ref());
}

/// Marks paths that don't exist on disk, to spot detection problems.
fn print_path(label: &str, path: Option<&PathBuf>) {
    match path {
        Some(path) if path.exists() => println!("  {label:<8}{}", path.display()),
        Some(path) => println!("  {label:<8}{} (no existe)", path.display()),
        None => println!("  {label:<8}-"),
    }
}
