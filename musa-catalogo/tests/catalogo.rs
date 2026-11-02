use std::collections::BTreeMap;
use std::fs;

use musa_catalogo::*;

fn config_vacia() -> BTreeMap<PedalId, PedalConfig> {
    BTreeMap::new()
}

fn preset_de_prueba(nombre: &str, creado: &str) -> Preset {
    let mut pedales = config_vacia();
    pedales.insert(
        PedalId::Reverb,
        PedalConfig {
            activo: true,
            valores: BTreeMap::from([("mezcla".to_string(), 0.4)]),
        },
    );
    Preset {
        version: VERSION_PRESET,
        id: format!("id-{nombre}"),
        nombre: nombre.to_string(),
        descripcion: "prueba".to_string(),
        peticion_original: "algo".to_string(),
        creado: creado.to_string(),
        pedales,
    }
}

#[test]
fn catalogo_tiene_todos_los_pedales_en_orden() {
    let ids: Vec<PedalId> = catalogo().iter().map(|p| p.id).collect();
    assert_eq!(ids, ORDEN_CADENA.to_vec());
}

#[test]
fn rangos_coherentes() {
    for pedal in catalogo() {
        assert!(
            !pedal.params.is_empty(),
            "{} no tiene parámetros",
            pedal.nombre
        );
        for p in pedal.params {
            assert!(p.min < p.max, "{}.{}: min >= max", pedal.nombre, p.clave);
            assert!(
                p.min <= p.defecto && p.defecto <= p.max,
                "{}.{}: defecto fuera de rango",
                pedal.nombre,
                p.clave
            );
        }
    }
}

#[test]
fn spec_devuelve_el_pedal_pedido() {
    for id in ORDEN_CADENA {
        assert_eq!(spec(id).id, id);
    }
}

#[test]
fn validar_recorta_fuera_de_rango() {
    let mut pedales = config_vacia();
    pedales.insert(
        PedalId::Reverb,
        PedalConfig {
            activo: true,
            valores: BTreeMap::from([("mezcla".to_string(), 500.0)]),
        },
    );
    validar(&mut pedales);
    let max = spec(PedalId::Reverb)
        .params
        .iter()
        .find(|p| p.clave == "mezcla")
        .expect("la reverb debe tener 'mezcla'")
        .max;
    assert_eq!(pedales[&PedalId::Reverb].valores["mezcla"], max);
}

#[test]
fn validar_rellena_faltantes_y_quita_desconocidos() {
    let mut pedales = config_vacia();
    pedales.insert(
        PedalId::Chorus,
        PedalConfig {
            activo: true,
            valores: BTreeMap::from([("inventada".to_string(), 1.0)]),
        },
    );
    validar(&mut pedales);

    for id in ORDEN_CADENA {
        assert!(pedales.contains_key(&id), "falta {id:?}");
    }
    let delay = &pedales[&PedalId::Delay];
    assert!(!delay.activo);
    for p in spec(PedalId::Delay).params {
        assert_eq!(delay.valores[p.clave], p.defecto);
    }
    let chorus = &pedales[&PedalId::Chorus];
    assert!(chorus.activo);
    assert!(!chorus.valores.contains_key("inventada"));
    assert_eq!(chorus.valores.len(), spec(PedalId::Chorus).params.len());
}

#[test]
fn validar_redondea_discretos() {
    let pedal = catalogo()
        .iter()
        .find(|p| p.params.iter().any(|x| x.discreto))
        .expect("al menos un parámetro discreto (forma del trémolo, ping-pong)");
    let param = pedal.params.iter().find(|x| x.discreto).unwrap();
    let mut pedales = config_vacia();
    pedales.insert(
        pedal.id,
        PedalConfig {
            activo: true,
            valores: BTreeMap::from([(param.clave.to_string(), param.min + 0.6)]),
        },
    );
    validar(&mut pedales);
    assert_eq!(pedales[&pedal.id].valores[param.clave], param.min + 1.0);
}

#[test]
fn describir_menciona_cada_pedal_y_rango() {
    let texto = describir_para_llm();
    for pedal in catalogo() {
        assert!(
            texto.contains(pedal.nombre),
            "falta el pedal {}",
            pedal.nombre
        );
        for p in pedal.params {
            assert!(texto.contains(p.clave), "falta el parámetro {}", p.clave);
        }
    }
}

#[test]
fn esquema_exige_campos() {
    let esquema = esquema_json();
    let requeridos: Vec<&str> = esquema["required"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    for campo in ["nombre", "descripcion", "prompt_icono", "pedales"] {
        assert!(requeridos.contains(&campo), "falta {campo} en required");
    }
    let pedales = esquema["properties"]["pedales"]["properties"]
        .as_object()
        .unwrap();
    for clave in [
        "compresor",
        "eq",
        "chorus",
        "phaser",
        "tremolo",
        "delay",
        "reverb",
    ] {
        assert!(pedales.contains_key(clave), "falta {clave} en el esquema");
    }
}

#[test]
fn slug_seguro() {
    assert_eq!(slug("Nébula Ñandú / 2!"), "nebula-nandu-2");
    assert_eq!(slug(""), "preset");
    assert_eq!(slug("  ¡¡¡  "), "preset");
}

#[test]
fn listar_ignora_corruptos() {
    let dir = tempfile::tempdir().unwrap();
    let b = preset_de_prueba("b", "2026-11-02T10:00:00Z");
    let a = preset_de_prueba("a", "2026-11-01T10:00:00Z");
    escribir_preset_atomico(&dir.path().join("b"), &b).unwrap();
    escribir_preset_atomico(&dir.path().join("a"), &a).unwrap();
    fs::create_dir(dir.path().join("roto")).unwrap();
    fs::write(
        dir.path().join("roto/preset.json"),
        "{\"version\": 1, \"id\": \"x",
    )
    .unwrap();

    let lista = listar_presets(dir.path());
    let nombres: Vec<&str> = lista.iter().map(|(_, p)| p.nombre.as_str()).collect();
    assert_eq!(nombres, ["a", "b"]);
    assert_eq!(lista[0].0, dir.path().join("a"));
}

#[test]
fn listar_dir_inexistente_es_vacio() {
    let dir = tempfile::tempdir().unwrap();
    assert!(listar_presets(&dir.path().join("no-existe")).is_empty());
}

#[test]
fn escritura_atomica_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let destino = dir.path().join("nebula");
    let p = preset_de_prueba("Nébula", "2026-11-01T10:00:00Z");
    escribir_preset_atomico(&destino, &p).unwrap();

    assert_eq!(leer_preset(&destino.join("preset.json")).unwrap(), p);
    let sobrantes: Vec<_> = fs::read_dir(&destino)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().ends_with(".tmp"))
        .collect();
    assert!(sobrantes.is_empty(), "quedó un archivo temporal");
}

#[test]
fn leer_preset_corrupto_da_error() {
    let dir = tempfile::tempdir().unwrap();
    let ruta = dir.path().join("preset.json");
    fs::write(&ruta, "no es json").unwrap();
    assert!(leer_preset(&ruta).is_err());
}

#[test]
fn preset_json_usa_claves_snake_case() {
    let p = preset_de_prueba("x", "2026-11-01T10:00:00Z");
    let json = serde_json::to_value(&p).unwrap();
    assert!(json["pedales"].get("reverb").is_some());
}
