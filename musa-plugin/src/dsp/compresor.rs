use std::collections::BTreeMap;

use super::{db_a_ganancia, valor, Pedal, Rampa};

pub struct Compresor {
    fs: f32,
    umbral_db: f32,
    ratio: f32,
    ataque_ms: f32,
    release_ms: f32,
    ganancia: Rampa,
    coef_ataque: f32,
    coef_release: f32,
    envolvente: f32,
}

impl Default for Compresor {
    fn default() -> Self {
        Self {
            fs: 48_000.0,
            umbral_db: -20.0,
            ratio: 1.0,
            ataque_ms: 10.0,
            release_ms: 100.0,
            ganancia: Rampa::nueva(30.0),
            coef_ataque: 0.0,
            coef_release: 0.0,
            envolvente: 0.0,
        }
    }
}

fn coeficiente(ms: f32, fs: f32) -> f32 {
    (-1.0 / (ms.max(0.1) / 1000.0 * fs)).exp()
}

impl Compresor {
    fn calcular_coeficientes(&mut self) {
        self.coef_ataque = coeficiente(self.ataque_ms, self.fs);
        self.coef_release = coeficiente(self.release_ms, self.fs);
    }
}

impl Pedal for Compresor {
    fn preparar(&mut self, frecuencia: f32, _max_bloque: usize) {
        self.fs = frecuencia;
        self.ganancia.preparar(frecuencia);
        self.calcular_coeficientes();
    }

    fn reiniciar(&mut self) {
        self.envolvente = 0.0;
        self.ganancia.saltar();
    }

    fn aplicar(&mut self, valores: &BTreeMap<String, f32>) {
        self.umbral_db = valor(valores, "umbral_db", -20.0);
        self.ratio = valor(valores, "ratio", 4.0).max(1.0);
        self.ataque_ms = valor(valores, "ataque_ms", 10.0);
        self.release_ms = valor(valores, "release_ms", 100.0);
        self.calcular_coeficientes();
        let ganancia = db_a_ganancia(valor(valores, "ganancia_db", 0.0));
        self.ganancia.objetivo(ganancia);
    }

    fn procesar(&mut self, izq: &mut [f32], der: &mut [f32]) {
        let pendiente = 1.0 - 1.0 / self.ratio;
        for (l, r) in izq.iter_mut().zip(der.iter_mut()) {
            let nivel = l.abs().max(r.abs());
            let coef = if nivel > self.envolvente {
                self.coef_ataque
            } else {
                self.coef_release
            };
            self.envolvente = nivel + coef * (self.envolvente - nivel);
            let nivel_db = 20.0 * self.envolvente.max(1e-6).log10();
            let reduccion_db = (nivel_db - self.umbral_db).max(0.0) * pendiente;
            let g = db_a_ganancia(-reduccion_db) * self.ganancia.siguiente();
            *l *= g;
            *r *= g;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsp::pruebas::*;

    const TIPICO: &[(&str, f32)] = &[
        ("umbral_db", -24.0),
        ("ratio", 4.0),
        ("ataque_ms", 10.0),
        ("release_ms", 120.0),
        ("ganancia_db", 6.0),
    ];

    #[test]
    fn ratio_uno_es_identidad() {
        comprobar_identidad(
            &mut Compresor::default(),
            &[("umbral_db", -30.0), ("ratio", 1.0), ("ganancia_db", 0.0)],
        );
    }

    #[test]
    fn estable_con_ruido() {
        comprobar_estable(&mut Compresor::default(), TIPICO);
    }

    #[test]
    fn sin_asignaciones_en_procesar() {
        comprobar_sin_asignaciones(&mut Compresor::default(), TIPICO);
    }

    #[test]
    fn compresor_reduce_picos() {
        let fs = 48_000.0;
        let mut pedal = Compresor::default();
        listo(
            &mut pedal,
            fs,
            &[
                ("umbral_db", -20.0),
                ("ratio", 4.0),
                ("ataque_ms", 5.0),
                ("release_ms", 100.0),
                ("ganancia_db", 0.0),
            ],
        );
        let mut izq = seno(440.0, 1.0, 9600, fs);
        let mut der = izq.clone();
        procesar(&mut pedal, &mut izq, &mut der, 64);
        let pico_db = 20.0 * pico(&izq[2400..]).log10();
        assert!(pico_db <= -10.0, "pico de {pico_db} dB");
    }
}
