use std::collections::BTreeMap;

use super::{valor, Pedal, Rampa};

const COMBS: [usize; 8] = [1116, 1188, 1277, 1356, 1422, 1491, 1557, 1617];
const ALLPASS: [usize; 4] = [556, 441, 341, 225];
const DESFASE_ESTEREO: usize = 23;
const GANANCIA_ENTRADA: f32 = 0.015;
const ESCALA_MOJADO: f32 = 2.0;
const REALIMENTACION_REFERENCIA: f32 = 0.825;

#[derive(Default)]
struct Comb {
    buffer: Vec<f32>,
    indice: usize,
    filtro: f32,
}

impl Comb {
    fn procesar(&mut self, x: f32, realimentacion: f32, amortiguacion: f32) -> f32 {
        let salida = self.buffer[self.indice];
        self.filtro = salida * (1.0 - amortiguacion) + self.filtro * amortiguacion;
        self.buffer[self.indice] = x + self.filtro * realimentacion;
        self.indice = (self.indice + 1) % self.buffer.len();
        salida
    }
}

#[derive(Default)]
struct Difusor {
    buffer: Vec<f32>,
    indice: usize,
}

impl Difusor {
    fn procesar(&mut self, x: f32) -> f32 {
        let guardado = self.buffer[self.indice];
        let salida = guardado - x;
        self.buffer[self.indice] = x + guardado * 0.5;
        self.indice = (self.indice + 1) % self.buffer.len();
        salida
    }
}

pub struct Reverb {
    combs: [Vec<Comb>; 2],
    difusores: [Vec<Difusor>; 2],
    tamano: Rampa,
    amortiguacion: Rampa,
    mezcla: Rampa,
}

impl Default for Reverb {
    fn default() -> Self {
        Self {
            combs: [Vec::new(), Vec::new()],
            difusores: [Vec::new(), Vec::new()],
            tamano: Rampa::nueva(50.0),
            amortiguacion: Rampa::nueva(50.0),
            mezcla: Rampa::nueva(30.0),
        }
    }
}

impl Pedal for Reverb {
    fn preparar(&mut self, frecuencia: f32, _max_bloque: usize) {
        let escala = frecuencia / 44_100.0;
        let largo = |base: usize, canal: usize| {
            (((base + canal * DESFASE_ESTEREO) as f32 * escala) as usize).max(1)
        };
        for canal in 0..2 {
            self.combs[canal] = COMBS
                .iter()
                .map(|&b| Comb {
                    buffer: vec![0.0; largo(b, canal)],
                    ..Comb::default()
                })
                .collect();
            self.difusores[canal] = ALLPASS
                .iter()
                .map(|&b| Difusor {
                    buffer: vec![0.0; largo(b, canal)],
                    ..Difusor::default()
                })
                .collect();
        }
        for rampa in [&mut self.tamano, &mut self.amortiguacion, &mut self.mezcla] {
            rampa.preparar(frecuencia);
        }
    }

    fn reiniciar(&mut self) {
        for comb in self.combs.iter_mut().flatten() {
            comb.buffer.fill(0.0);
            comb.filtro = 0.0;
        }
        for difusor in self.difusores.iter_mut().flatten() {
            difusor.buffer.fill(0.0);
        }
        for rampa in [&mut self.tamano, &mut self.amortiguacion, &mut self.mezcla] {
            rampa.saltar();
        }
    }

    fn aplicar(&mut self, valores: &BTreeMap<String, f32>) {
        self.tamano
            .objetivo(valor(valores, "tamano", 0.6).clamp(0.0, 1.0));
        self.amortiguacion
            .objetivo(valor(valores, "amortiguacion", 0.4).clamp(0.0, 1.0));
        self.mezcla
            .objetivo(valor(valores, "mezcla", 0.3).clamp(0.0, 1.0));
    }

    fn procesar(&mut self, izq: &mut [f32], der: &mut [f32]) {
        for (l, r) in izq.iter_mut().zip(der.iter_mut()) {
            let realimentacion = 0.7 + 0.25 * self.tamano.siguiente();
            let amortiguacion = 0.4 * self.amortiguacion.siguiente();
            let mezcla = self.mezcla.siguiente();
            let compensacion = ((1.0 - realimentacion * realimentacion)
                / (1.0 - REALIMENTACION_REFERENCIA * REALIMENTACION_REFERENCIA))
                .sqrt();
            let entrada = (*l + *r) * GANANCIA_ENTRADA;
            for (canal, muestra) in [l, r].into_iter().enumerate() {
                let mut s: f32 = self.combs[canal]
                    .iter_mut()
                    .map(|c| c.procesar(entrada, realimentacion, amortiguacion))
                    .sum();
                for difusor in self.difusores[canal].iter_mut() {
                    s = difusor.procesar(s);
                }
                *muestra = *muestra * (1.0 - mezcla) + s * ESCALA_MOJADO * compensacion * mezcla;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsp::pruebas::*;

    const TIPICO: &[(&str, f32)] = &[("tamano", 0.7), ("amortiguacion", 0.4), ("mezcla", 0.35)];

    #[test]
    fn mezcla_cero_es_identidad() {
        comprobar_identidad(
            &mut Reverb::default(),
            &[("tamano", 1.0), ("amortiguacion", 0.2), ("mezcla", 0.0)],
        );
    }

    #[test]
    fn estable_con_ruido() {
        comprobar_estable(
            &mut Reverb::default(),
            &[("tamano", 1.0), ("amortiguacion", 0.0), ("mezcla", 1.0)],
        );
    }

    #[test]
    fn sin_asignaciones_en_procesar() {
        comprobar_sin_asignaciones(&mut Reverb::default(), TIPICO);
    }

    #[test]
    fn reverb_tiene_cola() {
        let fs = 48_000.0;
        let mut pedal = Reverb::default();
        listo(&mut pedal, fs, TIPICO);
        let mut izq = impulso(48_000);
        let mut der = impulso(48_000);
        procesar(&mut pedal, &mut izq, &mut der, 256);
        let tramos: Vec<f32> = (0..4)
            .map(|i| energia(&izq[4_800 + i * 4_800..9_600 + i * 4_800]))
            .collect();
        assert!(tramos[0] > 0.0);
        assert!(
            tramos.windows(2).all(|w| w[1] < w[0]),
            "la cola no decae: {tramos:?}"
        );
    }
}
