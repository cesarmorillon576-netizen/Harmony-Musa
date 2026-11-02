# Harmony Musa — Plan de implementación

> **Para agentes:** SUB-SKILL REQUERIDA: usar superpowers:subagent-driven-development (recomendado) o
> superpowers:executing-plans para implementar este plan tarea por tarea. Los pasos usan casillas
> (`- [ ]`) para el seguimiento.

**Objetivo:** pedalera multiefectos de guitarra (plugin CLAP/VST3) cuyos presets diseña una IA local
desde una frase, con icono generado y aviso de voz.

**Arquitectura:** workspace de Cargo con `musa-catalogo` (fuente única de verdad de pedales y
presets), `musa-plugin` (nih-plug + nih_plug_egui), `ia-flujo` (Ollama → FastSD → TTS, un modelo
en RAM a la vez) y `musa-app` (eframe/egui). App y plugin se comunican solo por la carpeta
`~/.local/share/musa/presets/`.

**Tecnologías:** Rust 1.95, nih-plug (git), nih_plug_egui, eframe/egui, serde/serde_json, toml,
ureq (HTTP bloqueante), image, uuid, rodio, Ollama 0.40.13 (`qwen3.5:2b-q4_K_M`), FastSD CPU
(API REST), Qwen3-TTS 0.6B CustomVoice.

**Spec:** `docs/superpowers/specs/2026-10-31-musa-design.md`

## Restricciones globales

- **Git lo maneja el autor.** Quien ejecute el plan **nunca** hace `git commit` ni `git push`; los
  pasos "Commit" son sugerencias para el autor.
- Tareas marcadas **[AUTOR]** las programa el autor a mano: el ejecutor solo prepara tests/guía y
  revisa; **no escribe esa implementación**.
- Nunca dos modelos de IA cargados a la vez; cada paso libera su memoria antes del siguiente.
- Ollama: `think: false`, `num_ctx: 2048`, `keep_alive: 0`, `format` = esquema JSON del catálogo.
- `prompt_icono`: palabras clave en inglés separadas por comas, máx. ~50 palabras.
- Orden fijo de la cadena (9 pedales, `PEDALES = 9`): Puerta → Overdrive → Compresor → EQ → Chorus →
  Phaser → Trémolo → Delay → Reverb.
- Unidades por clave: `mezcla`, `profundidad`, `realimentacion`, `tamano`, `amortiguacion`, `tono`
  en 0–1; `*_db`, `*_ms`, `*_hz` en su unidad; `ratio` como proporción; `forma` y `ping_pong` 0/1.
- Salida estéreo; entrada mono o estéreo.
- Plugin: parámetro `Preset` entero 0…31; por pedal un selector `Según preset / Encendido / Apagado`
  (default `Según preset`); perillas no automatizables en v1.
- Cambios de preset y encendido/apagado sin clicks (rampa de ~10–30 ms).
- Hilo de audio: sin asignaciones de memoria, sin E/S de disco, sin bloqueos que puedan esperar.
- DSP objetivo: < ~10–15% de un núcleo a 48 kHz con los 7 pedales encendidos.
- Carpeta de presets: `~/.local/share/musa/presets/<slug>/{preset.json, icono.png}`.
- Formato de preset con `version` e `id` estable (UUID).
- Sin navegador/motor web, sin nube. Destino: Reaper en Linux (Hyprland/XWayland).
- Documentos y textos del repo en tono neutral (sin bitácora de conversación).

## Foco de revisión

Casos que el spec implica y que ningún flujo "feliz" ejercita; cada uno tiene su test en la tarea
indicada:

1. **Carpeta de presets vacía o inexistente** → el plugin deja pasar el audio sin cambios y la GUI
   dice "Sin presets"; nada de pánico. (Tarea 7)
2. **`preset.json` corrupto o a medio escribir** (la app escribe mientras el plugin recarga) → el
   plugin ignora ese archivo y carga los demás; la app escribe de forma atómica (temporal + rename).
   (Tareas 2 y 12)
3. **Automatización de `Preset` a un índice sin preset** (p. ej. 20 con 5 presets) → se mantiene el
   último preset válido, sin clicks ni silencio. (Tarea 7)
4. **Cambio de frecuencia de muestreo o tamaño de buffer** (44.1/48/96 kHz; 64–2048 muestras) →
   tiempos de delay correctos y sin asignaciones en `process`. (Tarea 5)
5. **Ollama apagado, puerto de FastSD ocupado o FastSD que muere a medias** → error claro en la
   app, ningún proceso huérfano y la app queda lista para reintentar. (Tarea 9)

---

## Calendario

| Día | Tareas |
|---|---|
| Dom 1 nov | 0, 1, 2 [AUTOR], 3 (compuerta de la GUI) |
| Lun 2 nov | 4, 5, 6 [AUTOR] |
| Mar 3 nov | 7, 8 · revisión del alcance |
| Mié 4 – Jue 5 | 9, 10, 11 [AUTOR], 12 |
| Vie 6 nov | 13 (demo) |
| Sáb 7 nov | holgura · 14 (MCP) si sobra tiempo |
| Dom 8 nov | entrega |

Si en la revisión del 3 de noviembre no se aprueba el alcance: ver "Contingencia" al final.

---

### Tarea 0: Entorno y mediciones

**Archivos:**
- Crear: `docs/mediciones.md`

**Interfaces:**
- Produce: en `docs/mediciones.md`, los campos exactos de la API de FastSD (`POST /api/generate`:
  JSON de entrada y campo de salida con la imagen en base64), el comando de arranque, el tiempo de
  la primera petición después de arrancar, tiempo por imagen 512×512 y RAM máxima.

- [ ] **Paso 1 (autor): instalar dependencias de sistema de nih-plug**

```bash
sudo apt install libasound2-dev libgl-dev libjack-dev libx11-xcb-dev libxcb1-dev \
  libxcb-dri2-0-dev libxcb-icccm4-dev libxcursor-dev libxkbcommon-dev \
  libxcb-shape0-dev libxcb-xfixes0-dev
```
Verificar: `pkg-config --exists jack x11-xcb && echo ok` imprime `ok`.

- [ ] **Paso 2 (autor): instalar FastSD CPU en `~/Proyectos/fastsdcpu`** (con buena conexión)

```bash
cd ~/Proyectos && git clone https://github.com/rupeshs/fastsdcpu && cd fastsdcpu
chmod +x install.sh && ./install.sh
```

- [ ] **Paso 3: medir FastSD**

Con Ollama detenido (`ollama stop qwen3.5:2b-q4_K_M`), arrancar `./start-webserver.sh`, abrir
`http://localhost:8000/api/docs`, anotar el esquema de `/api/generate`, generar 3 imágenes 512×512
con OpenVINO (SDXS-512 y, si sale mal, SD-Turbo) y anotar tiempos y RAM máxima en
`docs/mediciones.md`. Esperado: < 60 s por imagen y < 3.5 GB.

- [ ] **Paso 4: Commit (autor)** — `docs: mediciones de FastSD`

---

### Tarea 1: Workspace, xtask y esqueleto del catálogo con tests

**Archivos:**
- Crear: `Cargo.toml` (workspace: `musa-catalogo`, `musa-plugin`, `ia-flujo`, `musa-app`, `xtask`),
  `.cargo/config.toml` (alias `xtask = "run --package xtask --release --"`), `xtask/` (con
  `nih_plug_xtask`), `.gitignore` (`target/`).
- Crear: `musa-catalogo/Cargo.toml` (serde con derive, serde_json, uuid con v4+serde),
  `musa-catalogo/src/lib.rs`, `musa-catalogo/tests/catalogo.rs`.

**Interfaces (las define esta tarea; la Tarea 2 las implementa):**

```rust
// musa-catalogo/src/lib.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PedalId { Compresor, Eq, Chorus, Phaser, Tremolo, Delay, Reverb }

pub struct ParamSpec {
    pub clave: &'static str,        // "mezcla", "tiempo_ms", ...
    pub nombre: &'static str,       // texto visible
    pub min: f32, pub max: f32, pub defecto: f32,
    pub discreto: bool,             // se redondea al entero más cercano (formas de onda, ping-pong)
    pub descripcion: &'static str,  // qué hace en términos de sonido
}
pub struct PedalSpec {
    pub id: PedalId, pub nombre: &'static str,
    pub descripcion: &'static str, pub params: &'static [ParamSpec],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PedalConfig { pub activo: bool, pub valores: BTreeMap<String, f32> }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Preset {
    pub version: u32,                // VERSION_PRESET
    pub id: String,                  // UUID v4
    pub nombre: String, pub descripcion: String,
    pub peticion_original: String,
    pub creado: String,              // RFC 3339
    pub pedales: BTreeMap<PedalId, PedalConfig>,
}

/// Lo que devuelve el LLM.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PresetIa {
    pub nombre: String, pub descripcion: String, pub prompt_icono: String,
    pub pedales: BTreeMap<PedalId, PedalConfig>,
}

pub const VERSION_PRESET: u32 = 1;
pub const ORDEN_CADENA: [PedalId; 7];                       // orden de la cadena
pub fn catalogo() -> &'static [PedalSpec];                  // [AUTOR] Tarea 2
pub fn spec(id: PedalId) -> &'static PedalSpec;             // [AUTOR] Tarea 2
pub fn validar(pedales: &mut BTreeMap<PedalId, PedalConfig>); // [AUTOR] Tarea 2
pub fn describir_para_llm() -> String;                      // [AUTOR] Tarea 2
pub fn esquema_json() -> serde_json::Value;                 // Tarea 1 (ejecutor)
pub fn slug(nombre: &str) -> String;                        // Tarea 1 (ejecutor)
pub fn leer_preset(ruta: &Path) -> Result<Preset, String>;  // Tarea 1 (ejecutor)
pub fn listar_presets(dir: &Path) -> Vec<(PathBuf, Preset)>; // Tarea 1: ordenados por `creado`, ignora inválidos
pub fn escribir_preset_atomico(dir_preset: &Path, p: &Preset) -> std::io::Result<()>; // tmp + rename
```

Las funciones `[AUTOR]` quedan como `todo!()`; los tipos y las demás funciones, completos.

- [ ] **Paso 1: escribir los tests del catálogo** en `musa-catalogo/tests/catalogo.rs`
  - `catalogo_tiene_los_7_pedales_en_orden`: `catalogo().iter().map(|p| p.id)` == `ORDEN_CADENA`.
  - `rangos_coherentes`: para todo parámetro `min < max` y `min <= defecto <= max`.
  - `validar_recorta_fuera_de_rango`: reverb `mezcla` = 500 → queda en `max`.
  - `validar_rellena_faltantes_y_quita_desconocidos`: sin delay → delay `activo: false` con todos los
    defectos; clave `"inventada"` → eliminada.
  - `validar_redondea_discretos`: un parámetro discreto con 0.6 → 1.0.
  - `describir_menciona_cada_pedal_y_rango`: el texto contiene el nombre de cada pedal y cada `clave`.
  - `esquema_exige_campos`: `esquema_json()["required"]` contiene `nombre`, `descripcion`,
    `prompt_icono`, `pedales`; y `["properties"]["pedales"]["properties"]` tiene las 7 claves snake_case.
  - `slug_seguro`: `slug("Nébula Ñandú / 2!")` == `"nebula-nandu-2"`; `slug("")` == `"preset"`.
  - `listar_ignora_corruptos`: dir temporal con 2 presets válidos y un `preset.json` truncado →
    devuelve 2, ordenados por `creado`. Dir inexistente → vacío.
  - `escritura_atomica_roundtrip`: escribir y `leer_preset` devuelve el mismo `Preset`; no queda
    ningún `*.tmp` en el directorio.

- [ ] **Paso 2: implementar** tipos, `ORDEN_CADENA`, `esquema_json` (generado recorriendo
  `catalogo()`: por pedal `activo: boolean` y `valores` con `number` + `minimum`/`maximum`), `slug`
  (minúsculas, sin acentos, `[a-z0-9-]`), `leer_preset`, `listar_presets`,
  `escribir_preset_atomico`. Las `[AUTOR]` con `todo!()`.

- [ ] **Paso 3: verificar**
  Run: `cargo test -p musa-catalogo`
  Esperado: pasan `slug_seguro`, `listar_ignora_corruptos`, `escritura_atomica_roundtrip`; el resto
  falla con `not yet implemented` (son la tarea del autor). `esquema_exige_campos` también depende
  de `catalogo()`, así que falla hasta la Tarea 2.

- [ ] **Paso 4: Commit (autor)** — `feat: workspace y esqueleto del catálogo`

---

### Tarea 2 [AUTOR]: El catálogo de pedales

**Archivos:** Modificar `musa-catalogo/src/lib.rs` (las cuatro funciones `todo!()`).

**Interfaces:** Produce `catalogo`, `spec`, `validar`, `describir_para_llm` con las firmas de la Tarea 1.

**Guía (qué aprender):** `static` con slices de structs, `match` sobre enums, `BTreeMap`
(`entry`, `retain`), `f32::clamp`, `format!`/`String::push_str`, y cómo `#[derive(Serialize)]` +
`#[serde(rename_all)]` deciden el JSON.

- [ ] **Paso 1: la tabla** — escribir `catalogo()` con los 9 pedales y al menos estos parámetros
  (rangos y descripciones de sonido los elige el autor, respetando las unidades de las
  restricciones globales):
  - Puerta: `umbral_db`, `release_ms`
  - Overdrive: `ganancia_db`, `tono`, `mezcla`
  - Compresor: `umbral_db`, `ratio`, `ataque_ms`, `release_ms`, `ganancia_db`
  - EQ: `graves_db`, `medios_db`, `agudos_db`
  - Chorus: `velocidad_hz`, `profundidad`, `mezcla`
  - Phaser: `velocidad_hz`, `profundidad`, `realimentacion`, `mezcla`
  - Trémolo: `velocidad_hz`, `profundidad`, `forma` (discreto: 0 = seno, 1 = cuadrada)
  - Delay: `tiempo_ms` (máx. 1500), `realimentacion`, `mezcla`, `ping_pong` (discreto 0/1)
  - Reverb: `tamano`, `amortiguacion`, `mezcla`
- [ ] **Paso 2:** `spec(id)` y `validar` (rellenar faltantes con `defecto`, quitar claves
  desconocidas, recortar a `[min, max]`, redondear discretos).
- [ ] **Paso 3:** `describir_para_llm()` — un texto por pedal: nombre, descripción y cada parámetro
  con rango y qué significa sonoramente.
- [ ] **Paso 4:** `cargo test -p musa-catalogo` → todos pasan.
- [ ] **Paso 5: Commit** — `feat: catálogo de pedales`

---

### Tarea 3: Plugin mínimo + compuerta de la GUI en Reaper

**Archivos:**
- Crear: `musa-plugin/Cargo.toml` (`crate-type = ["cdylib", "lib"]`; `nih_plug` y `nih_plug_egui`
  por git desde `https://github.com/robbert-vdh/nih-plug`; si no compila con Rust 1.95, usar el
  fork comunitario `https://codeberg.org/BillyDM/nih-plug`), `musa-plugin/src/lib.rs`,
  `musa-plugin/src/gui.rs`, `scripts/instalar-plugin.sh`.

**Interfaces:**
- Produce: `struct Musa` con `nih_export_clap!` y `nih_export_vst3!`; nombre visible
  `"Harmony Musa"`, `CLAP_ID = "com.cesar.harmony-musa"`; ventana egui de 720×480.
- `scripts/instalar-plugin.sh`: `cargo xtask bundle musa-plugin --release` y copia
  `target/bundled/musa-plugin.clap` a `~/.clap/` y `.vst3` a `~/.vst3/`.

- [ ] **Paso 1:** plugin estéreo que deja pasar el audio sin cambios, con un parámetro de ganancia
  de prueba y una ventana egui con el título "Harmony Musa" y un `Label` con la frecuencia de muestreo.
- [ ] **Paso 2:** `time ./scripts/instalar-plugin.sh` — anotar en `docs/mediciones.md` la duración
  de la primera compilación y de una recompilación tras tocar `lib.rs`.
- [ ] **Paso 3 (compuerta, manual):** en Reaper (Hyprland) re-escanear plugins, insertar "Harmony
  Musa" en una pista con audio y abrir su ventana. Esperado: el audio pasa sin cortes, la ventana
  abre, se dibuja y responde al ratón; cerrarla y reabrirla 3 veces sin que Reaper se cuelgue.
  Si falla: probar `GDK_BACKEND=x11`/lanzar Reaper con `WAYLAND_DISPLAY=` y anotar el resultado.
  Si sigue fallando, avisar al autor antes de seguir (el respaldo del spec es vista genérica +
  iconos en la app).
- [ ] **Paso 4: Commit (autor)** — `feat: plugin mínimo de Harmony Musa`

---

### Tarea 4: Motor de la cadena (rasgo `Pedal` y fundidos sin clicks)

**Archivos:** Crear `musa-plugin/src/dsp/mod.rs`, `musa-plugin/src/dsp/cadena.rs`,
`musa-plugin/src/dsp/rampa.rs`; tests en los mismos archivos (`#[cfg(test)]`).

**Interfaces:**
```rust
pub trait Pedal: Send {
    fn preparar(&mut self, frecuencia: f32, max_bloque: usize); // único lugar donde se reserva memoria
    fn reiniciar(&mut self);
    fn aplicar(&mut self, valores: &BTreeMap<String, f32>);    // fuera de process o con valores ya validados
    fn procesar(&mut self, izq: &mut [f32], der: &mut [f32]);
}
pub struct Rampa;   // valor suavizado: fn nueva(ms: f32), fn objetivo(&mut self, v: f32), fn siguiente(&mut self) -> f32
pub struct Cadena;  // 7 pedales en ORDEN_CADENA, cada uno con una Rampa de mezcla seco/procesado
impl Cadena {
    pub fn nueva() -> Self;
    pub fn preparar(&mut self, frecuencia: f32, max_bloque: usize);
    pub fn encender(&mut self, id: PedalId, encendido: bool); // fundido de ~20 ms
    pub fn aplicar(&mut self, id: PedalId, valores: &BTreeMap<String, f32>);
    pub fn procesar(&mut self, izq: &mut [f32], der: &mut [f32]);
}
```
Mientras los pedales reales no existen, cada posición usa `PasaTodo` (implementa `Pedal` sin efecto).

- [ ] **Paso 1: tests**
  - `todo_apagado_es_identidad`: seno de 440 Hz, salida == entrada (tolerancia 1e-6).
  - `encender_no_produce_clicks`: con un pedal de prueba que invierte la señal, encenderlo a mitad de
    un seno de 0.5 de amplitud: la diferencia entre muestras consecutivas nunca supera 0.05.
  - `rampa_llega_al_objetivo`: a 48 kHz, 20 ms → llega al objetivo en ≤ 960 muestras.
- [ ] **Paso 2:** implementar. **Paso 3:** `cargo test -p musa-plugin dsp` → pasan.
- [ ] **Paso 4: Commit (autor)** — `feat: motor de la cadena con fundidos`

---

### Tarea 5: Pedales (compresor, EQ, chorus, phaser, delay, reverb)

**Archivos:** Crear `musa-plugin/src/dsp/{compresor,eq,chorus,phaser,delay,reverb}.rs`.

**Interfaces:** cada uno `pub struct X` + `impl Pedal`; las claves de `aplicar` son las de la
Tarea 2. Algoritmos: compresor feed-forward con detector de envolvente; EQ con biquads RBJ
(low-shelf 120 Hz, peaking 800 Hz Q 0.7, high-shelf 4 kHz); chorus con línea de retardo modulada
(7–25 ms) e interpolación lineal; phaser con 4–6 all-pass de primer orden modulados; delay estéreo
con ping-pong (buffer reservado para 1.5 s a 192 kHz en `preparar`); reverb Freeverb (8 comb + 4
all-pass por canal, desfasado en el derecho). Los valores se suavizan con `Rampa` para evitar
"zipper noise".

- [ ] **Paso 1: tests** (uno o más por pedal, en cada archivo)
  - Todos: `mezcla` 0 (o ganancias 0 dB en EQ) → identidad; ruido blanco de 1 s → todas las muestras
    finitas y `|x| < 4.0`.
  - `delay_impulso_en_su_tiempo`: impulso, `tiempo_ms` 250 a 48 kHz → primer eco en la muestra
    12000 ±1; mismo test a 44.1 kHz → 11025 ±1.
  - `delay_ping_pong_alterna`: primer eco solo en izquierda, segundo solo en derecha.
  - `reverb_tiene_cola`: impulso → energía entre 100 y 500 ms > 0 y decreciente.
  - `eq_realza_graves`: `graves_db` +12 → un seno de 80 Hz sale con ≥ +9 dB.
  - `compresor_reduce_picos`: seno de 0 dBFS, umbral −20 dB, ratio 4 → pico de salida (sin ganancia)
    ≤ −10 dBFS tras 50 ms.
  - `sin_asignaciones_en_procesar`: `preparar` y luego `procesar` con bloques de 64, 512 y 2048
    muestras a 44.1/48/96 kHz sin pánico (el ejecutor puede usar `assert_no_alloc` de nih-plug).
- [ ] **Paso 2:** implementar pedal por pedal hasta que pasen.
- [ ] **Paso 3: rendimiento** — test `#[ignore]` `cadena_completa_rendimiento`: 10 s de audio a 48 kHz
  con los 7 pedales encendidos en build `--release` debe tardar < 1.5 s.
  Run: `cargo test -p musa-plugin --release -- --ignored rendimiento`
- [ ] **Paso 4: Commit (autor)** — `feat: pedales de la cadena`

---

### Tarea 6 [AUTOR]: Pedal de trémolo

**Archivos:** Crear `musa-plugin/src/dsp/tremolo.rs`; reemplazar el `PasaTodo` de la posición del
trémolo en `cadena.rs`.

**Interfaces:** `pub struct Tremolo` + `impl Pedal` (Tarea 4). Claves: `velocidad_hz`,
`profundidad`, `forma`.

**Guía:** un oscilador de fase (`fase += velocidad / frecuencia`, módulo 1.0); ganancia
`1 - profundidad * (0.5 - 0.5 * cos(2π·fase))` para seno; para cuadrada, suavizar los bordes con
`Rampa` para que no haga clicks.

- [ ] **Paso 1 (ejecutor deja los tests listos):**
  - `profundidad_cero_es_identidad`.
  - `periodo_correcto`: `velocidad_hz` 4 a 48 kHz sobre una señal constante 1.0 → mínimos de
    ganancia separados 12000 ±2 muestras.
  - `profundidad_total_llega_a_cero`: con `profundidad` 1 la ganancia mínima < 0.01.
  - `cuadrada_sin_clicks`: diferencia entre muestras consecutivas < 0.05 sobre señal constante 0.5.
- [ ] **Paso 2 (autor):** implementar hasta que pasen: `cargo test -p musa-plugin tremolo`.
- [ ] **Paso 3:** probarlo en Reaper con la guitarra.
- [ ] **Paso 4: Commit** — `feat: pedal de trémolo`

---

### Tarea 7: Parámetros, presets y estado persistente del plugin

**Archivos:** Crear `musa-plugin/src/params.rs`, `musa-plugin/src/presets.rs`; modificar `lib.rs`.

**Interfaces:**
```rust
#[derive(Enum, PartialEq)] pub enum Modo { SegunPreset, Encendido, Apagado }
pub struct MusaParams {
    #[id = "preset"] pub preset: IntParam,          // 0..=31
    #[nested(array)] pub modos: [EnumParam<Modo>; 7], // ids "modo_compresor", ... en ORDEN_CADENA
    #[persist = "presets"] pub presets: Arc<RwLock<Vec<Preset>>>, // lo que se guarda con el proyecto
}
/// Conserva el orden de `guardados` (por id), actualiza su contenido si el archivo cambió,
/// agrega al final los nuevos de `en_disco` (orden por `creado`) y trunca a 32.
/// Los guardados que ya no existen en disco SE CONSERVAN (el proyecto sigue sonando igual).
pub fn fusionar(guardados: &[Preset], en_disco: &[Preset]) -> Vec<Preset>;
pub fn dir_presets() -> PathBuf; // ~/.local/share/musa/presets
```
Carga y fusión en `initialize()` y al pulsar "Recargar" (Tarea 8), nunca en `process`. El hilo de
audio lee la lista con `try_read`; si no puede, conserva el preset vigente. Al cambiar `preset` o un
`modo`, aplica valores con `Cadena::aplicar` y `Cadena::encender` (preset → `activo`; `Encendido` /
`Apagado` lo fuerzan).

- [ ] **Paso 1: tests de `fusionar`**
  - `nuevos_al_final`: guardados [A, B], disco [B, C, A] → [A, B, C].
  - `borrado_en_disco_se_conserva`: guardados [A, B], disco [B] → [A, B].
  - `contenido_actualizado`: B con otra `mezcla` en disco → se usa la versión del disco.
  - `tope_32`: 40 en disco y ninguno guardado → 32.
- [ ] **Paso 2: tests de aplicación** (sin host, llamando a la lógica que traduce parámetros a la
  cadena)
  - `sin_presets_es_identidad`: lista vacía → salida == entrada.
  - `indice_fuera_de_rango_mantiene_el_ultimo`: 5 presets, `preset` = 20 → sigue el preset vigente.
  - `modo_apagado_gana_al_preset` y `modo_encendido_gana_al_preset`.
- [ ] **Paso 3:** implementar. **Paso 4:** `cargo test -p musa-plugin` → pasan.
- [ ] **Paso 5 (manual):** en Reaper, con 3 presets de prueba escritos a mano en la carpeta,
  automatizar `preset` y `modo_delay`: los cambios se oyen y sin clicks; guardar el proyecto, borrar
  un preset del disco, reabrir → suena igual.
- [ ] **Paso 6: Commit (autor)** — `feat: presets y automatización`

---

### Tarea 8: GUI del plugin

**Archivos:** Modificar `musa-plugin/src/gui.rs`; crear `musa-plugin/src/iconos.rs`.

**Interfaces:** consume `MusaParams` y `fusionar`/`dir_presets` (Tarea 7). Iconos decodificados con
`image` a texturas egui, en caché por `id`, cargados en el hilo de la GUI (o en un hilo aparte) y
nunca en el de audio; si falta `icono.png`, un icono por defecto dibujado con egui.

- [ ] **Paso 1:** rejilla de presets (icono 96 px + nombre); clic → fija el parámetro `preset` con
  `ParamSetter` (para que Reaper lo registre); el activo resaltado.
- [ ] **Paso 2:** fila con los 7 pedales en orden: nombre, estado efectivo (encendido/apagado) y un
  selector de 3 posiciones ligado a `modos`.
- [ ] **Paso 3:** botón "Recargar" (vuelve a leer la carpeta y fusiona); con 0 presets muestra
  "Sin presets: genera uno con la app".
- [ ] **Paso 4 (manual):** en Reaper: copiar un preset nuevo a la carpeta, pulsar "Recargar" →
  aparece con su icono sin cortar el audio. Revisar que se vea bien (tema oscuro, espaciado limpio).
- [ ] **Paso 5: Commit (autor)** — `feat: interfaz del plugin`

---

### Tarea 9: `ia-flujo` — Ollama, FastSD y orquestación

**Archivos:** Crear `ia-flujo/Cargo.toml` (ureq con json, serde_json, base64, image),
`ia-flujo/src/{lib,ollama,fastsd,tts,proceso}.rs`.

**Interfaces:**
```rust
pub struct Config { pub ollama_url: String, pub modelo: String,
    pub fastsd_dir: PathBuf, pub fastsd_comando: String, pub fastsd_puerto: u16,
    pub tts: ConfigTts }
pub fn llm_json(cfg: &Config, sistema: &str, usuario: &str, esquema: &serde_json::Value)
    -> Result<serde_json::Value, ErrorFlujo>;            // /api/generate, opciones globales, keep_alive 0
pub struct ProcesoHijo;                                   // mata el proceso (y su grupo) en Drop
pub fn generar_imagen(cfg: &Config, prompt: &str) -> Result<image::RgbImage, ErrorFlujo>;
    // arranca FastSD, espera a que responda (sondeo con tiempo límite de 180 s), genera, lo mata
pub trait Voz { fn decir(&mut self, texto: &str) -> Result<Vec<f32>, ErrorFlujo>; } // mono, 24 kHz
#[derive(Debug)] pub enum ErrorFlujo { OllamaNoResponde, PuertoOcupado(u16), FastsdMurio, Tiempo, Otro(String) }
```
Usa los campos de la API anotados en la Tarea 0.

- [ ] **Paso 1: tests unitarios**
  - `peticion_ollama_correcta`: el JSON construido tiene `think:false`, `keep_alive:0`,
    `options.num_ctx:2048`, `stream:false` y `format` = el esquema dado.
  - `ollama_apagado_da_error`: URL `http://127.0.0.1:9` → `Err(OllamaNoResponde)` en < 3 s.
  - `puerto_ocupado_da_error`: ocupar el puerto con un `TcpListener` → `Err(PuertoOcupado)` y no se
    lanza FastSD.
  - `proceso_hijo_muere_en_drop`: lanzar `sleep 60` con `ProcesoHijo`, soltarlo → el PID ya no existe.
- [ ] **Paso 2: tests de integración `#[ignore]`** (con los servicios reales):
  `llm_devuelve_json_valido`, `fastsd_genera_512` (imagen 512×512 y ningún proceso de FastSD vivo
  al terminar: `pgrep -f fastsdcpu` vacío).
- [ ] **Paso 3:** implementar. **Paso 4:** `cargo test -p ia-flujo` y
  `cargo test -p ia-flujo -- --ignored` (con Ollama arriba) → pasan.
- [ ] **Paso 5: Commit (autor)** — `feat: flujo de IA local`

---

### Tarea 10: Voz — prueba acotada de `qwen3-tts-rs` y decisión

**Archivos:** Crear `ia-flujo/src/tts.rs` (implementación elegida) y, si hace falta,
`ia-flujo/sidecar_tts/{servidor.py,requirements.txt}`; anotar en `docs/mediciones.md`.

**Interfaces:** implementa `Voz` (Tarea 9). `ConfigTts { modo: Predefinida | Clonada,
voz: String, referencia_wav: Option<PathBuf>, referencia_texto: Option<String> }`.

- [ ] **Paso 1 (máx. ½ día):** con `qwen3-tts-rs`, generar "Tu preset Nébula está listo" con
  `Qwen3-TTS-12Hz-0.6B-CustomVoice` en CPU. Medir tiempo y RAM, y comprobar que al soltar el modelo
  el RSS del proceso baja al menos un 80% de lo que subió.
- [ ] **Paso 2: decisión.** Si genera voz inteligible y libera la RAM → se queda en proceso. Si no →
  sidecar Python (mismo patrón que FastSD: `ProcesoHijo`, puerto propio, se mata al terminar).
- [ ] **Paso 3: test `#[ignore]`** `voz_genera_audio`: devuelve > 0.5 s de muestras, todas finitas.
  El modo `Clonada` solo se valida (si falta `referencia_wav` → error claro); no se prueba de punta a punta.
- [ ] **Paso 4: Commit (autor)** — `feat: síntesis de voz`

---

### Tarea 11 [AUTOR]: Prompt de sistema para Qwen

**Archivos:** Crear `musa-app/src/prompt.rs` con `pub fn prompt_sistema() -> String`
(incluye `musa_catalogo::describir_para_llm()`).

**Guía:** explicar el rol ("diseñador de sonido para guitarra limpia post-amplificador"), que solo
puede usar los pedales del catálogo, cómo traducir palabras (espacial, surf, psicodélico, vintage,
agresivo) a pedales, que el nombre sea corto y evocador en español, y el formato de `prompt_icono`.

- [ ] **Paso 1 (ejecutor deja el test listo, `#[ignore]`, con Ollama arriba)** `prompts_dorados`:
  - "quiero un sonido espacial" → reverb activa con `mezcla` > 0.3 y delay activo.
  - "surf de los 60" → reverb activa y trémolo **o** delay corto activos.
  - "limpio y seco" → reverb y delay inactivos o con `mezcla` < 0.1.
  - Todos: `nombre` de 1–3 palabras; `prompt_icono` ≤ 50 palabras y sin acentos/ñ (inglés).
- [ ] **Paso 2 (autor):** escribir e iterar el prompt hasta que pase:
  `cargo test -p musa-app -- --ignored prompts_dorados`.
- [ ] **Paso 3: Commit** — `feat: prompt de sistema`

---

### Tarea 12: `musa-app` — la aplicación

**Archivos:** Crear `musa-app/Cargo.toml` (eframe, rodio, toml, image, uuid, chrono),
`musa-app/src/{main,config,generar,ui}.rs`, `config.ejemplo.toml`.

**Interfaces:**
```rust
pub enum Paso { Pensando, DibujandoIcono, Guardando, Hablando, Listo, Error(String) }
/// Corre en un hilo aparte y reporta cada paso por un canal.
pub fn generar_preset(cfg: &Config, peticion: &str, voz: &mut dyn Voz, avisar: impl Fn(Paso))
    -> Result<PathBuf, String>; // ruta de la carpeta del preset guardado
```
Orden: `llm_json` → `PresetIa` → `validar` → `generar_imagen(prompt_icono)` → redimensionar a 256×256
→ `escribir_preset_atomico` + `icono.png` → `voz.decir("Tu preset {nombre} está listo")` →
reproducir con rodio. Carpeta: `slug(nombre)`; si existe, sufijo `-2`, `-3`, … Si falla el icono
→ se guarda con el icono por defecto; si falla la voz → se guarda sin aviso; si falla el LLM →
`Error` y no se guarda nada.

- [ ] **Paso 1: tests** (con un `Voz` falso y funciones de LLM/imagen inyectadas)
  - `flujo_feliz_guarda_preset_e_icono`.
  - `falla_icono_igual_guarda`: `icono.png` es el icono por defecto.
  - `falla_voz_igual_guarda`.
  - `falla_llm_no_guarda`: la carpeta de presets queda igual.
  - `nombre_repetido_usa_sufijo`: dos veces "Nébula" → `nebula` y `nebula-2`.
  - `config_ejemplo_se_lee`: `config.ejemplo.toml` se deserializa sin error.
- [ ] **Paso 2:** implementar `generar_preset` (inyección de dependencias mediante closures o un trait).
- [ ] **Paso 3: UI** — campo de texto, botón "Generar" (deshabilitado mientras corre), indicador del
  paso, resultado (icono, nombre, pedales encendidos, botón "Escuchar aviso"), y lista de presets
  existentes con icono y botón "Borrar" (con confirmación).
- [ ] **Paso 4 (manual, de punta a punta):** con Zen cerrado, generar "quiero un sonido espacial":
  termina en < 3 min, la RAM disponible nunca baja de 500 MB (`free -m` en otra terminal), el preset
  aparece en el plugin tras "Recargar" y suena el aviso.
- [ ] **Paso 5: Commit (autor)** — `feat: aplicación Harmony Musa`

---

### Tarea 13: Demo

**Archivos:** Crear `docs/demo.md` (guion) y `README.md` (qué es, requisitos, instalación, uso,
capturas).

- [ ] **Paso 1 (autor):** generar 4–5 presets contrastantes (espacial, surf, psicodélico, limpio
  brillante, vintage).
- [ ] **Paso 2 (autor):** grabar en Reaper acordes y un punteo; automatizar `preset` y 2–3 `modo_*`.
- [ ] **Paso 3:** ensayo completo con la laptop recién iniciada: app → preset nuevo → recargar en el
  plugin → reproducir la demo. Anotar tiempos en `docs/demo.md`.
- [ ] **Paso 4: Commit (autor)** — `docs: guion de demo y README`

---

### Tarea 14 (extra): Servidor MCP

**Archivos:** Crear `musa-app/src/mcp.rs`; flag `musa-app --mcp` (stdio).

**Interfaces:** herramientas `listar_pedales` (devuelve `describir_para_llm()` + `esquema_json()`),
`listar_presets`, `crear_preset { nombre, descripcion, pedales, prompt_icono }` → usa `validar` y
la misma parte de `generar_preset` desde el icono en adelante.

- [ ] **Paso 1: tests:** `crear_preset` con valores fuera de rango → se guardan recortados;
  `listar_pedales` incluye los 7 pedales.
- [ ] **Paso 2:** implementar con el SDK oficial de MCP para Rust (`rmcp`).
- [ ] **Paso 3 (manual):** conectarlo desde un cliente MCP y crear un preset que aparezca en el plugin.
- [ ] **Paso 4: Commit (autor)** — `feat: servidor MCP`

---

## Contingencia (si no se aprueba el alcance el 3 de noviembre)

Se conserva `ia-flujo` (Tareas 9 y 10) y se crea `escenarios-app` según
`docs/superpowers/specs/2026-10-28-escenarios-ia-design.md`: misma estructura que la Tarea 12, con
salida `{prompt_imagen, frase}` guardada en `salidas/AAAA-MM-DD_HH-MM-SS/`. Ese día se escribe un
plan corto aparte.
