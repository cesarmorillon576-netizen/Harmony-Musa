# Mediciones en el equipo de desarrollo

Equipo: AMD Ryzen 3 3250U (2 núcleos / 4 hilos), 9.6 GiB de RAM, sin GPU usable para IA.

## LLM (Ollama 0.40.13, solo CPU)

| Modelo | Tiempo por respuesta (~90–110 tokens) | Velocidad | RAM del proceso |
|---|---|---|---|
| `qwen3.5:2b-q4_K_M` | ~45 s | ~3.3 tok/s | ~2.1 GB |
| `qwen3.5:4b-q4_K_M` | ~72 s | ~2.4–2.9 tok/s | ~4.0 GB |

## Compilación del plugin

| Acción | Tiempo |
|---|---|
| Primera compilación de `musa-plugin` (release, con dependencias) | 2 min 05 s |
| Empaquetado e instalación (`scripts/instalar-plugin.sh`, xtask incluido) | 39 s |
| Recompilación tras modificar `musa-plugin/src/lib.rs` | 8 s |

## FastSD CPU

Pendiente (Tarea 0).
