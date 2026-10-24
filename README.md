# osu-memory-reader

Lector de memoria de osu! stable en Rust, basado en [tosu](https://github.com/tosuapp/tosu).
Por ahora solo lee el beatmap seleccionado y lo imprime en la terminal cada vez que cambia, junto con las rutas en disco del `.osu`, su carpeta, el bg y el audio. Sin servidor, sin overlays, sin cálculo de PP.

Funciona en Windows y en Linux (osu! stable bajo Wine, por ejemplo con osu-winello).

```sh
cargo build --release
./target/release/osu-memory-reader          # imprime el mapa cada vez que cambia
./target/release/osu-memory-reader --once   # una sola lectura
```

En Windows el binario queda en `target\release\osu-memory-reader.exe`.

### Rutas

La carpeta del juego se detecta sola: en Windows, por la ruta de `osu!.exe`; en Linux, por el directorio de trabajo del proceso o el `osupath` de osu-winello. Si falla, se puede indicar con `OSU_PATH=/ruta/a/osu!`.

La carpeta Songs se lee de los ajustes de osu! (`BeatmapDirectory`). Bajo Wine, las rutas tipo `D:\Songs` se traducen a la ruta real con los `dosdevices` del prefix.

### Permisos

- **Linux:** si no encuentra el patrón, ejecútalo con `sudo` o dale la capacidad:
  ```sh
  sudo setcap cap_sys_ptrace=eip target/release/osu-memory-reader
  ```
- **Windows:** si osu! corre como administrador, este programa también tiene que hacerlo.

## Estructura

- `src/process/mod.rs`: lecturas tipadas, strings de .NET y escaneo de patrones (común a ambas plataformas)
- `src/process/linux.rs`: `/proc` y `process_vm_readv`
- `src/process/windows.rs`: `CreateToolhelp32Snapshot`, `ReadProcessMemory` y `VirtualQueryEx`
- `src/pattern.rs`: patrones de bytes con comodines `??`
- `src/stable.rs`: patrones y offsets del beatmap y la carpeta Songs en osu! stable
- `src/paths.rs`: construye las rutas del juego, Songs, `.osu`, bg y audio
- `src/enums.rs`: nombres de modos y estados de ranked
