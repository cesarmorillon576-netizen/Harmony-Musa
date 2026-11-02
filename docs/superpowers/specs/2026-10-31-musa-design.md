# Harmony Musa — Documento de diseño

Fecha: 2026-10-31 · Autor: César

Alternativa: si este alcance no se aprueba, se implementa el diseño de
`2026-10-28-escenarios-ia-design.md`. Ambos comparten el flujo LLM → imagen → voz (sección 5).

## 1. Qué es y por qué

**Harmony Musa** es una **pedalera multiefectos para guitarra en forma de plugin de audio**
(CLAP/VST3) acompañada de una **app nativa** donde describes con palabras el sonido que buscas
(*"quiero un sonido espacial"*) y una IA local diseña un **preset**: qué pedales se encienden y con
qué valores, un nombre y un **icono** generado. Al terminar, una **voz** avisa que el preset está
listo. Los presets se acumulan como archivos y aparecen en el plugin, igual que en una pedalera
multiefectos real.

Usa tres modelos locales con un propósito concreto: **Qwen** para diseñar la configuración,
generación de imagen para el icono y síntesis de voz (con clonación preparada) para el aviso.
Revisión del alcance: **martes 3 de noviembre**. Entrega: **8 de noviembre de 2026**.

### La demo (lo que define el éxito)
Se graban en Reaper líneas de guitarra (acordes y un punteo). Al reproducirlas, la
**automatización de Reaper** va cambiando de preset de Musa y encendiendo/apagando pedales, y se
escucha la diferencia. Antes, se muestra la app generando un preset nuevo desde una frase: aparece con
su icono en el plugin y la voz lo anuncia.

### Objetivos
- El plugin carga en **Reaper en Linux** y procesa audio en tiempo real sin cortes.
- Cambiar de preset y encender/apagar cada pedal es **automatizable** desde Reaper.
- La app genera un preset válido + icono + aviso de voz, 100% local, con un modelo en RAM a la vez.
- Los sonidos de distintos presets se distinguen claramente al oído.

### Anti-objetivos (fracaso aunque "funcione")
- Clicks, cortes o picos de CPU que arruinen la demo al cambiar de preset.
- Que el LLM pueda producir presets inválidos (pedales inexistentes, valores fuera de rango).
- Navegador/motor web, APIs en la nube, o dos modelos de IA cargados a la vez.

### No-objetivos
- Que el LLM escriba o compile código de plugins (imposible en esta máquina; ver sección 3).
- Simulación completa de amplificador (el overdrive incluye solo un filtro tipo bocina para tocar en directo).
- Edición manual de perillas desde la interfaz del plugin con automatización (v1 solo enciende/apaga
  pedales y cambia de preset; ver sección 6.3).
- Soportar otras máquinas, Windows/macOS, o DAWs distintos de Reaper.
- Instalar modelos desde la app (se dejan preparados de antemano).
- Clonación de voz funcional (solo preparada por configuración, como en el diseño original).

### Extra si sobra tiempo
- **Servidor MCP** dentro de la app (sección 8). No es indispensable para la entrega.

## 2. Contexto de uso

- Cadena de señal: `guitarra → iRig → Reaper → [Musa]` (directo) o con el procesador Mooer
  (ampli + gabinete) antes del iRig. Musa funciona en ambos casos; en directo, la puerta de ruido y
  el overdrive con filtro de bocina dan la ganancia.
- DAW: **Reaper** en Linux (Hyprland/Wayland), ya instalado y en uso (con PipeWire vía `pw-jack`).

## 3. Restricciones de hardware

- **AMD Ryzen 3 3250U** (2 núcleos / 4 hilos), **sin GPU usable** → toda la IA en CPU.
- **RAM 9.6 GiB**, ~4–6 GiB libres según lo abierto. Zen con pestañas usa ~1.6 GB.
- **Medido:** `qwen3.5:2b-q4_K_M` ~45 s por respuesta corta y ~2.1 GB; el 4B ~72 s y ~4 GB (presiona swap).
- Por eso: un modelo cargado a la vez, nada de compilar en caliente, app nativa ligera.
- El plugin corre dentro de Reaper al mismo tiempo que la app: su DSP debe ser barato
  (objetivo: < ~10–15% de un núcleo a 48 kHz con todos los pedales encendidos).

## 4. Arquitectura general

Workspace de Cargo con tres crates :

| Crate | Tipo | Qué hace |
|---|---|---|
| `musa-catalogo` | biblioteca | **Única fuente de verdad**: los pedales, sus parámetros, rangos y descripciones; el formato del preset (serde); validación/clamp; generación del esquema JSON para el LLM y de la descripción en texto para prompts/MCP |
| `musa-plugin` | `cdylib` (nih-plug) | El plugin CLAP + VST3: DSP de los pedales, parámetros automatizables, lista de presets con icono (GUI con `nih_plug_egui`) |
| `musa-app` | binario (eframe/egui) | La app: escribir la petición, orquestar LLM → icono → voz, guardar el preset en la carpeta compartida. Más adelante, servidor MCP |

Además, un crate **`ia-flujo`** (biblioteca) con lo que comparten Musa y el plan B: cliente de
Ollama, encender/apagar FastSD, TTS y la regla de "un modelo en RAM a la vez". `musa-app` lo usa; si
hubiera que volver al plan B, sería otra app delgada en el mismo workspace usando `ia-flujo`. El plan
cubre Musa + `ia-flujo`; el plan B solo como contingencia breve.

Carpeta compartida de presets: `~/.local/share/musa/presets/<slug>/` con `preset.json` e `icono.png`.
La app escribe ahí; el plugin lee de ahí.

## 5. La app (flujo de generación)

```
[el usuario escribe "quiero un sonido espacial"] → Generar
  1. LLM (Ollama, qwen3.5:2b)   → JSON validado contra el esquema del catálogo:
        { nombre, descripcion, pedales: {…}, prompt_icono }
     → descargar modelo (keep_alive: 0)
  2. Icono (FastSD CPU, API :8000) → arrancar → PNG 512×512 → reducir a icono → matar proceso
  3. Guardar preset.json + icono.png en la carpeta compartida
  4. Voz (Qwen3-TTS 0.6B CustomVoice) → "Tu preset Nebulosa está listo" → liberar modelo
  5. Mostrar en la app: icono, nombre, qué pedales usa; reproducir el aviso
```

Reglas (heredadas del diseño original):
- **Nunca dos modelos de IA cargados a la vez.** Cada paso libera memoria antes del siguiente.
- LLM: `format` = **esquema JSON generado desde el catálogo** (Ollama structured outputs), `think: false`,
  `num_ctx` 2048, `keep_alive: 0`. El prompt de sistema incluye la descripción del catálogo
  (qué hace cada pedal y qué significan sus rangos en términos de sonido).
- La app **valida y ajusta (clamp)** todo lo que devuelva el LLM con `musa-catalogo` antes de guardar.
- `prompt_icono`: palabras clave en inglés separadas por comas, máx. ~50 palabras, estilo icono
  (ej. "nebula, stars, glowing, icon, centered, minimal").
- Generación en segundo plano; la UI muestra el paso (Pensando → Dibujando icono → Guardando → Hablando)
  y no se congela. Si un paso falla: mostrar error, liberar lo cargado (matar FastSD si quedó vivo),
  quedar lista para reintentar. Si falla solo el icono o la voz, **el preset igual se guarda**
  (con un icono por defecto / sin aviso).
- La app también lista los presets existentes (con icono) y permite borrar uno.
- Configuración en `config.toml`: URL/modelo de Ollama, comando/puerto de FastSD y modelo de imagen,
  modo de TTS (predefinida/clonada) y voz, ruta de la carpeta de presets.
- Velocidad aceptable: hasta ~2 min por preset (aceptable para la demo).

Componentes de IA (iguales al diseño original): Ollama 0.40.13 ya instalado con `qwen3.5:2b-q4_K_M`
(default) y `4b`; FastSD CPU con SDXS-512 o SD-Turbo vía API REST (pendiente de instalar y medir);
Qwen3-TTS 0.6B CustomVoice: primero prueba acotada (≤ ½ día) de `qwen3-tts-rs` en proceso; si no
genera voz en CPU o no libera la RAM, sidecar Python con encender/apagar como FastSD.

## 6. El plugin

### 6.1 Pedales (en este orden en la cadena; 9 en total)

| # | Pedal | Parámetros mínimos (rangos exactos a criterio del constructor) |
|---|---|---|
| 0a | Puerta de ruido | umbral (dB), release (ms) |
| 0b | Overdrive | ganancia (dB), tono, mezcla; incluye filtro tipo bocina |
| 1 | Compresor | umbral, ratio, ataque, release, ganancia de salida |
| 2 | EQ | graves, medios, agudos (shelving/peaking) |
| 3 | Chorus | velocidad, profundidad, mezcla |
| 4 | Phaser | velocidad, profundidad, realimentación, mezcla |
| 5 | Trémolo | velocidad, profundidad, forma de onda (seno/cuadrada) |
| 6 | Delay | tiempo (ms), realimentación, mezcla, ping-pong sí/no |
| 7 | Reverb | tamaño, amortiguación (damping), mezcla |

Entrada mono o estéreo; salida **estéreo** (el ping-pong y la reverb la necesitan).

### 6.2 Preset (formato, en `musa-catalogo`)
`preset.json`: `id` (estable, p. ej. UUID), `nombre`, `descripcion`, `peticion_original`, `creado` (fecha), y por cada pedal
`activo: bool` + sus valores. Versión del formato incluida para poder cambiarlo después.

### 6.3 Parámetros automatizables (lo que ve Reaper)
- **`Preset`**: entero 0…N-1 que selecciona un preset de la lista (lista ordenada por fecha de
  creación al cargar el plugin; tope fijo, p. ej. 32). Cambiarlo aplica los valores de ese preset.
- **Por cada pedal, un selector de 3 estados: `Según preset` / `Encendido` / `Apagado`**
  (default `Según preset`). Así la automatización puede forzar a encender o apagar cualquier pedal
  encima de lo que diga el preset.
- Los valores de las perillas vienen del preset y no se exponen como automatizables en v1.
- **Estabilidad del proyecto de Reaper**: el plugin guarda en su estado
  persistente (lo que Reaper guarda con el proyecto) la lista de presets en uso con su `id` y sus
  valores. Al reabrir el proyecto, cada posición de `Preset` sigue apuntando al mismo sonido aunque
  luego se borren o agreguen presets en la carpeta; los presets nuevos se agregan al final de la lista.
- Cambios de preset y de encendido/apagado deben ser **suaves** (rampa/crossfade de unos ms) para no
  producir clicks. Delay y reverb pueden dejar sonar su cola al apagarse o cortarla; a criterio del
  constructor, pero sin clicks.

### 6.4 Interfaz del plugin (nih_plug_egui)
Lista/rejilla de presets con **icono y nombre**; el activo resaltado; la cadena de 7 pedales con su
estado (encendido/apagado) visible; botón "Recargar" para leer presets nuevos de la carpeta.
Carga de presets e iconos **fuera del hilo de audio**.

**La GUI del plugin es indispensable**: la demo muestra el
preset nuevo con su icono dentro del plugin. Por eso abrir la ventana de `nih_plug_egui` en Reaper
sobre Hyprland/XWayland es una **prueba obligatoria del primer día**, junto al plugin mínimo. Solo si
resulta imposible después de intentarlo, el respaldo es la vista genérica de Reaper + iconos en la app.

## 7. División del trabajo

El autor implementa a mano las piezas con más valor de aprendizaje (~2–3 h disponibles); el resto
se implementa con asistencia de IA y se documenta lo importante de cada parte.
- **A mano (autor):** el **catálogo de pedales** en `musa-catalogo` (structs/enums/serde), el
  **pedal de trémolo** completo y el **prompt de sistema** para Qwen.
- **Asistido:** estructura de nih-plug, parámetros y carga de presets/iconos; los demás pedales;
  orquestación de IA y manejo de memoria; la interfaz egui de la app; el MCP si sobra tiempo.
- Para no bloquear el resto, se dejan primero las **firmas** que los demás crates necesitan
  (p. ej. `validar/clamp`, `esquema_json`, `describir_para_llm`) como esqueleto con `todo!()`; el
  autor escribe los tipos de los pedales y del preset y rellena esas funciones. El trémolo arranca
  como pedal "pasa-todo" (sin efecto) hasta que se implemente.
- El plan marca qué tareas son del autor y las deja como guías (qué aprender, qué escribir, cómo
  probarlo), no como código hecho.

## 8. Extra: servidor MCP

La app expone un servidor MCP (stdio o HTTP local, a criterio del constructor) para que cualquier IA
(Claude, OpenCode, etc.) cree presets:
- `listar_pedales` → el catálogo con descripciones y rangos (sale de `musa-catalogo`).
- `listar_presets` → los presets existentes.
- `crear_preset(nombre, descripcion, pedales…)` → valida, genera el icono, guarda y anuncia con voz.
Mismo catálogo, misma validación y mismo flujo de icono/voz que la app.

## 9. Orden sugerido (por el calendario)

El autor puede programar **desde el 1 de noviembre**: sus tareas (catálogo, prompt, trémolo) pueden ir
temprano, en paralelo con el resto.

- **Hasta el martes 3:** lo que sirve para Musa y para el plan B: medir FastSD y TTS, y el flujo
  LLM → imagen → voz en la app. Empezar el catálogo y el esqueleto del plugin cargando en Reaper.
- **Del 3 al 8:** si se aprueba el alcance, pedales, presets, automatización, GUI del plugin y demo grabada.
  Si no, terminar la versión de escenarios.
- Hito temprano de riesgo: un plugin nih-plug mínimo (ganancia) **compilando y cargando en Reaper en
  esta máquina**, y medir cuánto tarda compilar en el Ryzen 3.

## 10. A criterio del constructor
Crates concretos (cliente HTTP, MCP, audio), algoritmos de cada efecto (p. ej. Freeverb para la
reverb), rangos y curvas de los parámetros, slug/nombres de carpeta, estilo visual (sencillo y limpio),
cómo detectar que FastSD está listo, tamaño final del icono.

## 11. Riesgos
- DSP de calidad (reverb, chorus, phaser) es la parte más difícil; usar algoritmos clásicos y probados.
- Compilar nih-plug en esta máquina puede ser lento; medirlo pronto.
- Plugin en Wayland/Reaper: la GUI de nih_plug_egui usa X11/XWayland vía baseview; verificar pronto.
- `qwen3-tts-rs` es joven; hay sidecar Python como respaldo.
- Solo 5 días entre la revisión del alcance y la entrega.
