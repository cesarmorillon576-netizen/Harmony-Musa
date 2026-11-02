# Escenarios IA — Documento de diseño

Fecha: 2026-10-28 · Autor: César · Estado: alternativa a Harmony Musa

## 1. Qué es y por qué

Una **app de escritorio nativa en Rust** donde el usuario escribe un
escenario corto (ej. *"persona perdida en un bosque"*) y la app produce:

- una **imagen** de la escena, y
- un **audio** con una frase dicha por una voz (ej. alguien pidiendo ayuda).

Todo corre **100% local** en una laptop modesta. Usa modelos **Qwen** e incluye, si es posible, **clonación de voz**. La prioridad de diseño es **consumir la menor memoria posible**:
por eso es nativa (no navegador) y por eso los modelos se cargan **uno a la vez**, en flujo.

**Fecha de entrega: 8 de noviembre de 2026** (~11 días desde este documento).

### Éxito
- Escribir un escenario, pulsar "Generar" y obtener imagen + audio sin que la laptop se quede sin RAM.
- Poder mostrar resultados guardados en disco.
- Se entrega y ya: no es un producto de uso continuo.

### Anti-objetivos (sería un fracaso aunque "funcione")
- Que dos modelos estén cargados en RAM al mismo tiempo y la máquina empiece a usar swap.
- Que la interfaz se congele mientras se genera.
- Usar un navegador / motor web (Tauri, Electron, servidor web + pestaña).
- Depender de internet o de APIs en la nube.

### No-objetivos
- Instalar o descargar modelos desde la app (el usuario los deja preparados).
- Uso multiusuario, acceso remoto, API HTTP pública.
- Soportar otras máquinas/GPU; basta con que funcione en la del autor.
- Edición de imágenes, historial navegable, varias frases/personajes por escenario.

## 2. Restricciones de hardware (máquina objetivo)

- CPU **AMD Ryzen 3 3250U** (2 núcleos / 4 hilos, 15 W, AVX2). **Sin GPU usable** (Vega 3 iGPU,
  2 GB de RAM reservada, sin ROCm) → toda la inferencia es en **CPU**.
- **RAM 9.6 GiB** visibles, ~6 GiB libres en uso normal. Swap 6.8 GiB (evitar tocarlo).
- Disco: ~58 GB libres.
- Linux (Ubuntu-based, kernel 7.0), **Hyprland/Wayland**. Python 3.12 disponible.
- La laptop se calienta con carga sostenida; el uso será por ratos.

## 3. Componentes (decididos)

| Paso | Componente | Modelo | RAM aprox. | Cómo se usa |
|---|---|---|---|---|
| 1. Texto | **Ollama** 0.40.13 (instalado manualmente en `/usr/local`, sin servicio systemd) | `qwen3.5:2b-q4_K_M` (alternativa `qwen3.5:4b-q4_K_M`) | ~2.1 GB (4B: ~4 GB) | API HTTP local de Ollama |
| 2. Imagen | **FastSD CPU** (Python, OpenVINO) | SDXS-512 o SD-Turbo, 512×512 | ~2–3 GB | Proceso aparte que la app enciende y apaga; se usa por su API |
| 3. Voz | **Qwen3-TTS** | `Qwen3-TTS-12Hz-0.6B-CustomVoice` (voz predefinida) | ~2 GB | Preferido: en el mismo proceso vía crate `qwen3-tts-rs`. Fallback: sidecar Python |

- **LLM medido en la máquina real (2026-10-29/30):** todo en CPU (`size_vram: 0`).
  - 4B: ~2.4–2.9 tok/s, ~72 s por escenario, ~4 GB de RAM; con Zen abierto empuja el sistema a swap.
  - 2B: ~3.3 tok/s, ~45 s por escenario, ~2.1 GB de RAM, sin presión de swap. Sigue algo peor las
    instrucciones, pero el resultado es aceptable → **2B es el modelo por defecto**; el modelo es
    configurable. Calidad final se juzga ya con la app.
- Velocidades aún **estimadas**: imagen 5–15 s; TTS ~4–6× la duración del audio. El usuario acepta
  hasta ~2 min por escenario.
- **Opciones del LLM:** `num_ctx` 2048, `think: false`, `format: json`, `keep_alive: 0`.
- **Prompt de imagen:** el LLM debe producir **palabras clave en inglés separadas por comas, máx.
  ~50 palabras** (CLIP trunca a ~77 tokens y no entiende instrucciones en prosa como "to imply
  presence without revealing faces").
- **Clonación de voz:** se decide después (de quién es la voz). **Para esta entrega solo se deja
  preparada**, no tiene que funcionar ni demostrarse: el diseño debe permitir cambiar a
  `Qwen3-TTS-12Hz-0.6B-Base` + audio de referencia + su transcripción **solo por configuración**.
  Riesgo aceptado: ese camino puede no haberse probado nunca antes de activarse.
- **Ruta del TTS** *(por la fecha de entrega)*: primero una prueba acotada
  (máx. ~medio día) de `qwen3-tts-rs` en el mismo proceso, verificando que genere voz con
  CustomVoice en CPU y que al liberarlo la RAM regrese al sistema. Si falla cualquiera de las dos,
  se usa el sidecar Python con el mismo patrón encender/apagar que FastSD, sin más intentos.

## 4. Flujo

```
[usuario escribe escenario] → Generar
  1. LLM (Ollama, qwen3.5)  → JSON { prompt_imagen (inglés), frase (español) }
     → descargar modelo de RAM (keep_alive: 0)
  2. Imagen (FastSD CPU)     → arrancar proceso → generar PNG → matar proceso
  3. Voz (Qwen3-TTS)          → cargar modelo → generar WAV → liberar modelo
  4. Mostrar imagen, reproducir audio, guardar todo en salidas/
```

Reglas:
- **Nunca dos modelos cargados a la vez.** Cada paso libera su memoria antes de iniciar el siguiente.
- El LLM debe devolver salida estructurada (JSON) con los dos campos; desactivar el modo
  *thinking* de Qwen3.5 para velocidad.
- La generación corre en un hilo/tarea en segundo plano; la UI muestra el paso actual
  (Pensando → Dibujando → Hablando) y no se congela.
- Si un paso falla, se muestra el error en la UI, se libera lo que haya quedado cargado
  (incluido matar el proceso de FastSD) y la app queda lista para reintentar.
- Mientras hay una generación en curso, el botón Generar está deshabilitado (una a la vez).

## 5. App (Rust)

- **UI: egui / eframe** (decisión tomada por ligereza y rapidez de desarrollo). Debe funcionar en Wayland.
- **Audio:** reproducción dentro de la app (ej. `rodio`).
- **Estructura:** una **biblioteca** con el flujo (cliente Ollama, control de FastSD, TTS, pipeline)
  y una **interfaz egui delgada** encima, para poder añadir otra interfaz después sin reescribir.
- **Interfaz mínima:** campo de texto para el escenario, botón Generar, indicador de paso/progreso,
  la imagen resultante, botón para reproducir/repetir el audio, y la frase generada como texto.
- **Configuración:** archivo simple (ej. `config.toml`) con: URL de Ollama y nombre del modelo,
  ruta/comando de FastSD CPU y su puerto, modelo de imagen, modo de TTS (predefinida/clonada),
  voz predefinida, ruta del audio de referencia + transcripción (para clonación), carpeta de salidas.
- **Salidas:** cada ejecución guarda en `salidas/AAAA-MM-DD_HH-MM-SS/`: `imagen.png`, `voz.wav` y
  `texto.json` (escenario original, prompt de imagen, frase).

## 6. Lo que queda a criterio del constructor

- Crates concretos (cliente HTTP, async vs. hilos, serialización).
- Cómo detectar que FastSD ya está listo para recibir peticiones tras arrancarlo.
- Texto exacto del prompt de sistema para el LLM (debe producir un prompt visual en inglés y una
  frase corta en español coherente con el escenario).
- Si `qwen3-tts-rs` no funciona bien en CPU/esta máquina, usar el sidecar Python con el mismo
  patrón encender/apagar que FastSD.
- Estilo visual de la UI (sencillo y limpio basta).

## 7. Preparación del entorno (la hace el usuario; el plan solo la verifica)

El usuario instala y deja listos los runtimes y modelos; la app no instala nada. Al momento de este
documento **Ollama ya está instalado y medido** (ver sección 3). El primer hito del plan debe ser una
lista de verificación + medición de lo que falta: FastSD CPU (versión con modo API REST;
confirmar versión y endpoint), y Qwen3-TTS CustomVoice (en Rust o en el entorno Python del sidecar).
Cada paso se mide por separado (tiempo y RAM máxima) antes de integrar.

## 8. Riesgos conocidos

- Las velocidades son estimadas; el primer hito debería medir cada paso por separado en la máquina real.
- `qwen3-tts-rs` es joven; puede no soportar la variante CustomVoice o ir lento en CPU.
- FastSD CPU cambia de modelo por defecto entre versiones (SDXS vs. SD-Turbo); fijarlo en configuración.
