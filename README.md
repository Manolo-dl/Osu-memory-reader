# osu-memory-reader

Lector de memoria de osu! stable en Rust, basado en [tosu](https://github.com/tosuapp/tosu).
Por ahora solo lee el beatmap seleccionado y lo imprime en la terminal cada vez que cambia: sin servidor, sin overlays, sin cálculo de PP.

Funciona en Windows y en Linux (osu! stable bajo Wine, por ejemplo con osu-winello).

```sh
cargo build --release
./target/release/osu-memory-reader          # imprime el mapa cada vez que cambia
./target/release/osu-memory-reader --once   # una sola lectura
```

En Windows el binario queda en `target\release\osu-memory-reader.exe`.

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
- `src/stable.rs`: patrón y offsets del beatmap en osu! stable
- `src/enums.rs`: nombres de modos y estados de ranked
