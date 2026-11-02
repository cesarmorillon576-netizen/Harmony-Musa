use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const VERSION_PRESET: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PedalId {
    Puerta,
    Overdrive,
    Compresor,
    Eq,
    Chorus,
    Phaser,
    Tremolo,
    Delay,
    Reverb,
}

pub const PEDALES: usize = 9;

pub const ORDEN_CADENA: [PedalId; PEDALES] = [
    PedalId::Puerta,
    PedalId::Overdrive,
    PedalId::Compresor,
    PedalId::Eq,
    PedalId::Chorus,
    PedalId::Phaser,
    PedalId::Tremolo,
    PedalId::Delay,
    PedalId::Reverb,
];

#[derive(Debug)]
pub struct ParamSpec {
    pub clave: &'static str,
    pub nombre: &'static str,
    pub min: f32,
    pub max: f32,
    pub defecto: f32,
    pub discreto: bool,
    pub descripcion: &'static str,
}

#[derive(Debug)]
pub struct PedalSpec {
    pub id: PedalId,
    pub nombre: &'static str,
    pub descripcion: &'static str,
    pub params: &'static [ParamSpec],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PedalConfig {
    pub activo: bool,
    pub valores: BTreeMap<String, f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Preset {
    pub version: u32,
    pub id: String,
    pub nombre: String,
    pub descripcion: String,
    pub peticion_original: String,
    pub creado: String,
    pub pedales: BTreeMap<PedalId, PedalConfig>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PresetIa {
    pub nombre: String,
    pub descripcion: String,
    pub prompt_icono: String,
    pub pedales: BTreeMap<PedalId, PedalConfig>,
}

pub fn catalogo() -> &'static [PedalSpec] {
    todo!("Tarea 2: tabla de pedales")
}

pub fn spec(id: PedalId) -> &'static PedalSpec {
    let _ = id;
    todo!("Tarea 2: buscar el pedal en catalogo()")
}

pub fn validar(pedales: &mut BTreeMap<PedalId, PedalConfig>) {
    let _ = pedales;
    todo!("Tarea 2: validar y recortar")
}

pub fn describir_para_llm() -> String {
    todo!("Tarea 2: describir el catálogo")
}

pub fn esquema_json() -> serde_json::Value {
    use serde_json::{json, Map, Value};

    let mut pedales = Map::new();
    for pedal in catalogo() {
        let mut valores = Map::new();
        for p in pedal.params {
            let tipo = if p.discreto { "integer" } else { "number" };
            valores.insert(
                p.clave.to_string(),
                json!({ "type": tipo, "minimum": p.min, "maximum": p.max }),
            );
        }
        let claves: Vec<&str> = pedal.params.iter().map(|p| p.clave).collect();
        let nombre = serde_json::to_value(pedal.id).expect("PedalId siempre serializa");
        let nombre = nombre
            .as_str()
            .expect("PedalId serializa como texto")
            .to_string();
        pedales.insert(
            nombre,
            json!({
                "type": "object",
                "properties": {
                    "activo": { "type": "boolean" },
                    "valores": { "type": "object", "properties": valores, "required": claves }
                },
                "required": ["activo", "valores"]
            }),
        );
    }
    let requeridos: Vec<Value> = pedales.keys().map(|k| Value::from(k.as_str())).collect();
    json!({
        "type": "object",
        "properties": {
            "nombre": { "type": "string" },
            "descripcion": { "type": "string" },
            "prompt_icono": { "type": "string" },
            "pedales": { "type": "object", "properties": pedales, "required": requeridos }
        },
        "required": ["nombre", "descripcion", "prompt_icono", "pedales"]
    })
}

pub fn slug(nombre: &str) -> String {
    use unicode_normalization::UnicodeNormalization;

    let mut salida = String::new();
    let mut guion_pendiente = false;
    for c in nombre.nfd().filter(|c| c.is_ascii() || c.is_alphanumeric()) {
        if c.is_ascii_alphanumeric() {
            if guion_pendiente && !salida.is_empty() {
                salida.push('-');
            }
            guion_pendiente = false;
            salida.push(c.to_ascii_lowercase());
        } else if !c.is_alphanumeric() {
            guion_pendiente = true;
        }
    }
    if salida.is_empty() {
        "preset".to_string()
    } else {
        salida
    }
}

pub fn leer_preset(ruta: &Path) -> Result<Preset, String> {
    let texto = std::fs::read_to_string(ruta).map_err(|e| format!("{}: {e}", ruta.display()))?;
    serde_json::from_str(&texto).map_err(|e| format!("{}: {e}", ruta.display()))
}

pub fn listar_presets(dir: &Path) -> Vec<(PathBuf, Preset)> {
    let Ok(entradas) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut presets: Vec<(PathBuf, Preset)> = entradas
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|ruta| ruta.is_dir())
        .filter_map(|ruta| {
            let preset = leer_preset(&ruta.join("preset.json")).ok()?;
            Some((ruta, preset))
        })
        .collect();
    presets.sort_by(|a, b| (&a.1.creado, &a.1.id).cmp(&(&b.1.creado, &b.1.id)));
    presets
}

pub fn escribir_preset_atomico(dir_preset: &Path, preset: &Preset) -> std::io::Result<()> {
    std::fs::create_dir_all(dir_preset)?;
    let temporal = dir_preset.join("preset.json.tmp");
    let json = serde_json::to_string_pretty(preset).map_err(std::io::Error::other)?;
    std::fs::write(&temporal, json)?;
    std::fs::rename(&temporal, dir_preset.join("preset.json"))
}
