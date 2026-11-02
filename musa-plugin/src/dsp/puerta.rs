use std::collections::BTreeMap;

use super::{valor, Pedal};

const HISTERESIS_DB: f32 = 6.0;
const ATAQUE_MS: f32 = 1.0;
const SOSTEN_MS: f32 = 15.0;
const DETECTOR_MS: f32 = 10.0;
const PISO: f32 = 1e-4;

pub struct Puerta {
    fs: f32,
    umbral_db: f32,
    release_ms: f32,
    envolvente: f32,
    ganancia: f32,
    abierta: bool,
    sosten: usize,
    coef_detector: f32,
    coef_ataque: f32,
    coef_release: f32,
}

impl Default for Puerta {
    fn default() -> Self {
        Self {
            fs: 48_000.0,
            umbral_db: -60.0,
            release_ms: 80.0,
            envolvente: 0.0,
            ganancia: 1.0,
            abierta: true,
            sosten: 0,
            coef_detector: 0.0,
            coef_ataque: 0.0,
            coef_release: 0.0,
        }
    }
}

fn coeficiente(ms: f32, fs: f32) -> f32 {
    (-1.0 / (ms.max(0.1) / 1000.0 * fs)).exp()
}

impl Puerta {
    fn calcular(&mut self) {
        self.coef_detector = coeficiente(DETECTOR_MS, self.fs);
        self.coef_ataque = coeficiente(ATAQUE_MS, self.fs);
        self.coef_release = coeficiente(self.release_ms, self.fs);
    }
}

impl Pedal for Puerta {
    fn preparar(&mut self, frecuencia: f32, _max_bloque: usize) {
        self.fs = frecuencia;
        self.calcular();
    }

    fn reiniciar(&mut self) {
        self.envolvente = 0.0;
        self.ganancia = 1.0;
        self.abierta = true;
        self.sosten = (SOSTEN_MS / 1000.0 * self.fs) as usize;
    }

    fn aplicar(&mut self, valores: &BTreeMap<String, f32>) {
        self.umbral_db = valor(valores, "umbral_db", -60.0);
        self.release_ms = valor(valores, "release_ms", 80.0);
        self.calcular();
    }

    fn procesar(&mut self, izq: &mut [f32], der: &mut [f32]) {
        let abrir = 10f32.powf(self.umbral_db / 20.0);
        let cerrar = 10f32.powf((self.umbral_db - HISTERESIS_DB) / 20.0);
        let sosten_total = (SOSTEN_MS / 1000.0 * self.fs) as usize;
        for (l, r) in izq.iter_mut().zip(der.iter_mut()) {
            let nivel = l.abs().max(r.abs());
            self.envolvente = if nivel > self.envolvente {
                nivel
            } else {
                nivel + self.coef_detector * (self.envolvente - nivel)
            };
            if self.envolvente >= abrir {
                self.abierta = true;
                self.sosten = sosten_total;
            } else if self.envolvente < cerrar {
                if self.sosten > 0 {
                    self.sosten -= 1;
                } else {
                    self.abierta = false;
                }
            }
            let (objetivo, coef) = if self.abierta {
                (1.0, self.coef_ataque)
            } else {
                (0.0, self.coef_release)
            };
            self.ganancia = objetivo + coef * (self.ganancia - objetivo);
            if !self.abierta && self.ganancia < PISO {
                self.ganancia = 0.0;
            }
            *l *= self.ganancia;
            *r *= self.ganancia;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsp::pruebas::*;

    const FS: f32 = 48_000.0;

    #[test]
    fn umbral_minimo_es_identidad() {
        comprobar_identidad(
            &mut Puerta::default(),
            &[("umbral_db", -100.0), ("release_ms", 80.0)],
        );
    }

    #[test]
    fn estable_con_ruido() {
        comprobar_estable(
            &mut Puerta::default(),
            &[("umbral_db", -20.0), ("release_ms", 50.0)],
        );
    }

    #[test]
    fn sin_asignaciones_en_procesar() {
        comprobar_sin_asignaciones(
            &mut Puerta::default(),
            &[("umbral_db", -50.0), ("release_ms", 80.0)],
        );
    }

    #[test]
    fn senal_fuerte_pasa() {
        let mut pedal = Puerta::default();
        listo(
            &mut pedal,
            FS,
            &[("umbral_db", -40.0), ("release_ms", 80.0)],
        );
        let original = seno(220.0, 0.3, 9600, FS);
        let (mut izq, mut der) = (original.clone(), original.clone());
        procesar(&mut pedal, &mut izq, &mut der, 64);
        let pico_final = pico(&izq[4800..]);
        assert!((pico_final - 0.3).abs() < 0.01, "pico {pico_final}");
    }

    #[test]
    fn zumbido_debajo_del_umbral_se_calla() {
        let mut pedal = Puerta::default();
        listo(
            &mut pedal,
            FS,
            &[("umbral_db", -40.0), ("release_ms", 50.0)],
        );
        let original = seno(60.0, 0.001, 24_000, FS);
        let (mut izq, mut der) = (original.clone(), original.clone());
        procesar(&mut pedal, &mut izq, &mut der, 64);
        assert!(pico(&izq[12_000..]) < 1e-5, "el zumbido sigue sonando");
    }

    #[test]
    fn cerrar_no_produce_clicks() {
        let mut pedal = Puerta::default();
        listo(
            &mut pedal,
            FS,
            &[("umbral_db", -40.0), ("release_ms", 60.0)],
        );
        let mut izq = seno(220.0, 0.3, 24_000, FS);
        for x in izq[9_600..].iter_mut() {
            *x *= 0.002;
        }
        let mut der = izq.clone();
        procesar(&mut pedal, &mut izq, &mut der, 64);
        assert!(
            salto_maximo(&izq[9_600..]) < 0.01,
            "salto {}",
            salto_maximo(&izq[9_600..])
        );
        assert!(pico(&izq[20_000..]) < 1e-4);
    }
}
