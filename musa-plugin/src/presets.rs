use std::path::PathBuf;

use musa_catalogo::{Preset, ORDEN_CADENA, PEDALES};

use crate::dsp::Cadena;
use crate::params::Modo;

pub const MAXIMO_PRESETS: usize = 32;

pub fn dir_presets() -> PathBuf {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .unwrap_or_else(|| {
            let home = std::env::var_os("HOME")
                .map(PathBuf::from)
                .unwrap_or_default();
            home.join(".local/share")
        });
    base.join("musa/presets")
}

pub fn fusionar(guardados: &[Preset], en_disco: &[Preset]) -> Vec<Preset> {
    let mut resultado: Vec<Preset> = guardados
        .iter()
        .map(|g| en_disco.iter().find(|d| d.id == g.id).unwrap_or(g).clone())
        .collect();
    let mut nuevos: Vec<&Preset> = en_disco
        .iter()
        .filter(|d| !guardados.iter().any(|g| g.id == d.id))
        .collect();
    nuevos.sort_by(|a, b| (&a.creado, &a.id).cmp(&(&b.creado, &b.id)));
    resultado.extend(nuevos.into_iter().cloned());
    resultado.truncate(MAXIMO_PRESETS);
    resultado
}

pub fn etiqueta_preset(presets: &[Preset], indice: i32) -> String {
    usize::try_from(indice)
        .ok()
        .and_then(|i| presets.get(i))
        .map(|p| p.nombre.clone())
        .unwrap_or_else(|| format!("{} · vacío", indice + 1))
}

pub struct Aplicador {
    vigente: Option<usize>,
    modos: [Modo; PEDALES],
    efectivo: [bool; PEDALES],
    sucio: bool,
}

impl Default for Aplicador {
    fn default() -> Self {
        Self {
            vigente: None,
            modos: [Modo::SegunPreset; PEDALES],
            efectivo: [false; PEDALES],
            sucio: true,
        }
    }
}

impl Aplicador {
    pub fn invalidar(&mut self) {
        self.sucio = true;
    }

    pub fn actualizar(
        &mut self,
        cadena: &mut Cadena,
        presets: &[Preset],
        indice: usize,
        modos: [Modo; PEDALES],
    ) {
        let nuevo = if indice < presets.len() {
            Some(indice)
        } else {
            self.vigente.filter(|&v| v < presets.len())
        };
        let cambio_preset = self.sucio || nuevo != self.vigente;
        if !cambio_preset && modos == self.modos {
            return;
        }
        if cambio_preset {
            if let Some(i) = nuevo {
                for (id, config) in &presets[i].pedales {
                    cadena.aplicar(*id, &config.valores);
                }
            }
        }
        for (k, id) in ORDEN_CADENA.iter().enumerate() {
            let del_preset = nuevo
                .and_then(|i| presets[i].pedales.get(id))
                .is_some_and(|c| c.activo);
            let encendido = match modos[k] {
                Modo::SegunPreset => del_preset,
                Modo::Encendido => true,
                Modo::Apagado => false,
            };
            if self.sucio || encendido != self.efectivo[k] {
                cadena.encender(*id, encendido);
            }
            self.efectivo[k] = encendido;
        }
        self.vigente = nuevo;
        self.modos = modos;
        self.sucio = false;
    }

    pub fn vigente(&self) -> Option<usize> {
        self.vigente
    }

    pub fn efectivo(&self) -> [bool; PEDALES] {
        self.efectivo
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use musa_catalogo::{PedalConfig, PedalId, VERSION_PRESET};

    use super::*;
    use crate::dsp::pruebas::seno;

    fn preset(id: &str, creado: &str, activos: &[(PedalId, f32)]) -> Preset {
        let pedales = activos
            .iter()
            .map(|(pedal, mezcla)| {
                (
                    *pedal,
                    PedalConfig {
                        activo: true,
                        valores: BTreeMap::from([("mezcla".to_string(), *mezcla)]),
                    },
                )
            })
            .collect();
        Preset {
            version: VERSION_PRESET,
            id: id.to_string(),
            nombre: id.to_uppercase(),
            descripcion: String::new(),
            peticion_original: String::new(),
            creado: creado.to_string(),
            pedales,
        }
    }

    fn ids(presets: &[Preset]) -> Vec<&str> {
        presets.iter().map(|p| p.id.as_str()).collect()
    }

    fn posicion(id: PedalId) -> usize {
        ORDEN_CADENA.iter().position(|p| *p == id).unwrap()
    }

    const SEGUN: [Modo; PEDALES] = [Modo::SegunPreset; PEDALES];

    #[test]
    fn nuevos_al_final() {
        let a = preset("a", "2026-11-01T00:00:00Z", &[]);
        let b = preset("b", "2026-11-02T00:00:00Z", &[]);
        let c = preset("c", "2026-11-03T00:00:00Z", &[]);
        let resultado = fusionar(&[a.clone(), b.clone()], &[b, c, a]);
        assert_eq!(ids(&resultado), ["a", "b", "c"]);
    }

    #[test]
    fn nuevos_ordenados_por_fecha() {
        let x = preset("x", "2026-11-05T00:00:00Z", &[]);
        let y = preset("y", "2026-11-04T00:00:00Z", &[]);
        assert_eq!(ids(&fusionar(&[], &[x, y])), ["y", "x"]);
    }

    #[test]
    fn borrado_en_disco_se_conserva() {
        let a = preset("a", "2026-11-01T00:00:00Z", &[]);
        let b = preset("b", "2026-11-02T00:00:00Z", &[]);
        let resultado = fusionar(&[a.clone(), b.clone()], &[b]);
        assert_eq!(ids(&resultado), ["a", "b"]);
        assert_eq!(resultado[0], a);
    }

    #[test]
    fn contenido_actualizado() {
        let viejo = preset("b", "2026-11-02T00:00:00Z", &[(PedalId::Reverb, 0.2)]);
        let nuevo = preset("b", "2026-11-02T00:00:00Z", &[(PedalId::Reverb, 0.9)]);
        let resultado = fusionar(&[viejo], &[nuevo.clone()]);
        assert_eq!(resultado, vec![nuevo]);
    }

    #[test]
    fn tope_32() {
        let disco: Vec<Preset> = (0..40)
            .map(|i| {
                preset(
                    &format!("p{i:02}"),
                    &format!("2026-11-01T00:00:{i:02}Z"),
                    &[],
                )
            })
            .collect();
        let resultado = fusionar(&[], &disco);
        assert_eq!(resultado.len(), MAXIMO_PRESETS);
        assert_eq!(resultado[0].id, "p00");
    }

    #[test]
    fn dir_presets_bajo_local_share() {
        assert!(
            dir_presets().ends_with(".local/share/musa/presets")
                || dir_presets().ends_with("musa/presets")
        );
    }

    #[test]
    fn sin_presets_es_identidad() {
        let mut cadena = Cadena::completa();
        cadena.preparar(48_000.0, 512);
        let mut aplicador = Aplicador::default();
        aplicador.actualizar(&mut cadena, &[], 0, SEGUN);
        let original = seno(440.0, 0.5, 4800, 48_000.0);
        let (mut izq, mut der) = (original.clone(), original.clone());
        cadena.procesar(&mut izq, &mut der);
        assert_eq!(izq, original);
        assert_eq!(aplicador.vigente(), None);
        assert_eq!(aplicador.efectivo(), [false; PEDALES]);
    }

    #[test]
    fn preset_enciende_sus_pedales() {
        let mut cadena = Cadena::completa();
        cadena.preparar(48_000.0, 512);
        let mut aplicador = Aplicador::default();
        let presets = [preset(
            "a",
            "2026-11-01T00:00:00Z",
            &[(PedalId::Delay, 0.4)],
        )];
        aplicador.actualizar(&mut cadena, &presets, 0, SEGUN);
        assert_eq!(aplicador.vigente(), Some(0));
        assert!(aplicador.efectivo()[posicion(PedalId::Delay)]);
        assert!(cadena.encendido(PedalId::Delay));
        assert!(!cadena.encendido(PedalId::Reverb));
    }

    #[test]
    fn indice_fuera_de_rango_mantiene_el_ultimo() {
        let mut cadena = Cadena::completa();
        cadena.preparar(48_000.0, 512);
        let mut aplicador = Aplicador::default();
        let presets: Vec<Preset> = (0..5)
            .map(|i| {
                preset(
                    &format!("p{i}"),
                    "2026-11-01T00:00:00Z",
                    &[(ORDEN_CADENA[i], 0.5)],
                )
            })
            .collect();
        aplicador.actualizar(&mut cadena, &presets, 2, SEGUN);
        let antes = aplicador.efectivo();
        aplicador.actualizar(&mut cadena, &presets, 20, SEGUN);
        assert_eq!(aplicador.vigente(), Some(2));
        assert_eq!(aplicador.efectivo(), antes);
    }

    #[test]
    fn modo_apagado_gana_al_preset() {
        let mut cadena = Cadena::completa();
        cadena.preparar(48_000.0, 512);
        let mut aplicador = Aplicador::default();
        let presets = [preset(
            "a",
            "2026-11-01T00:00:00Z",
            &[(PedalId::Reverb, 0.5)],
        )];
        let mut modos = SEGUN;
        modos[posicion(PedalId::Reverb)] = Modo::Apagado;
        aplicador.actualizar(&mut cadena, &presets, 0, modos);
        assert!(!aplicador.efectivo()[posicion(PedalId::Reverb)]);
    }

    #[test]
    fn modo_encendido_gana_al_preset() {
        let mut cadena = Cadena::completa();
        cadena.preparar(48_000.0, 512);
        let mut aplicador = Aplicador::default();
        let presets = [preset(
            "a",
            "2026-11-01T00:00:00Z",
            &[(PedalId::Reverb, 0.5)],
        )];
        let mut modos = SEGUN;
        modos[posicion(PedalId::Chorus)] = Modo::Encendido;
        aplicador.actualizar(&mut cadena, &presets, 0, modos);
        assert!(aplicador.efectivo()[posicion(PedalId::Chorus)]);
        assert!(cadena.encendido(PedalId::Chorus));
    }

    #[test]
    fn actualizar_sin_cambios_no_asigna() {
        let mut cadena = Cadena::completa();
        cadena.preparar(48_000.0, 512);
        let mut aplicador = Aplicador::default();
        let presets = [preset(
            "a",
            "2026-11-01T00:00:00Z",
            &[(PedalId::Delay, 0.4)],
        )];
        aplicador.actualizar(&mut cadena, &presets, 0, SEGUN);
        assert_no_alloc::assert_no_alloc(|| {
            aplicador.actualizar(&mut cadena, &presets, 0, SEGUN);
            let mut modos = SEGUN;
            modos[0] = Modo::Encendido;
            aplicador.actualizar(&mut cadena, &presets, 0, modos);
        });
    }

    #[test]
    fn etiqueta_muestra_el_nombre() {
        let presets = [
            preset("a", "2026-11-01T00:00:00Z", &[]),
            preset("b", "2026-11-02T00:00:00Z", &[]),
        ];
        assert_eq!(etiqueta_preset(&presets, 1), "B");
        assert_eq!(etiqueta_preset(&presets, 7), "8 · vacío");
        assert_eq!(etiqueta_preset(&[], 0), "1 · vacío");
    }
}
