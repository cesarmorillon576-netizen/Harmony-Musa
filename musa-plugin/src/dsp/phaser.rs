use std::collections::BTreeMap;
use std::f32::consts::{PI, TAU};

use super::{valor, Pedal, Rampa};

const ETAPAS: usize = 6;
const FRECUENCIA_MINIMA: f32 = 200.0;
const FRECUENCIA_MAXIMA: f32 = 2_000.0;

pub struct Phaser {
    fs: f32,
    estados: [[f32; ETAPAS]; 2],
    retorno: [f32; 2],
    fase: f32,
    velocidad: f32,
    profundidad: Rampa,
    realimentacion: Rampa,
    mezcla: Rampa,
}

impl Default for Phaser {
    fn default() -> Self {
        Self {
            fs: 48_000.0,
            estados: [[0.0; ETAPAS]; 2],
            retorno: [0.0; 2],
            fase: 0.0,
            velocidad: 0.5,
            profundidad: Rampa::nueva(30.0),
            realimentacion: Rampa::nueva(30.0),
            mezcla: Rampa::nueva(30.0),
        }
    }
}

impl Pedal for Phaser {
    fn preparar(&mut self, frecuencia: f32, _max_bloque: usize) {
        self.fs = frecuencia;
        self.profundidad.preparar(frecuencia);
        self.realimentacion.preparar(frecuencia);
        self.mezcla.preparar(frecuencia);
    }

    fn reiniciar(&mut self) {
        self.estados = [[0.0; ETAPAS]; 2];
        self.retorno = [0.0; 2];
        self.fase = 0.0;
        for rampa in [
            &mut self.profundidad,
            &mut self.realimentacion,
            &mut self.mezcla,
        ] {
            rampa.saltar();
        }
    }

    fn aplicar(&mut self, valores: &BTreeMap<String, f32>) {
        self.velocidad = valor(valores, "velocidad_hz", 0.5);
        self.profundidad
            .objetivo(valor(valores, "profundidad", 0.7).clamp(0.0, 1.0));
        self.realimentacion
            .objetivo(valor(valores, "realimentacion", 0.4).clamp(0.0, 0.9));
        self.mezcla
            .objetivo(valor(valores, "mezcla", 0.5).clamp(0.0, 1.0));
    }

    fn procesar(&mut self, izq: &mut [f32], der: &mut [f32]) {
        let rango = (FRECUENCIA_MAXIMA / FRECUENCIA_MINIMA).ln();
        for (l, r) in izq.iter_mut().zip(der.iter_mut()) {
            let profundidad = self.profundidad.siguiente();
            let realimentacion = self.realimentacion.siguiente();
            let mezcla = self.mezcla.siguiente();
            for (canal, muestra) in [l, r].into_iter().enumerate() {
                let lfo = 0.5 + 0.5 * (self.fase * TAU + canal as f32 * PI * 0.5).sin();
                let frecuencia = FRECUENCIA_MINIMA * (rango * profundidad * lfo).exp();
                let t = (PI * frecuencia / self.fs).tan();
                let coef = (t - 1.0) / (t + 1.0);
                let mut s = *muestra + self.retorno[canal] * realimentacion;
                for estado in self.estados[canal].iter_mut() {
                    let y = coef * s + *estado;
                    *estado = s - coef * y;
                    s = y;
                }
                self.retorno[canal] = s;
                let compensado = s * (1.0 - 0.5 * realimentacion);
                *muestra += (compensado - *muestra) * mezcla;
            }
            self.fase = (self.fase + self.velocidad / self.fs).fract();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsp::pruebas::*;

    const TIPICO: &[(&str, f32)] = &[
        ("velocidad_hz", 0.5),
        ("profundidad", 0.8),
        ("realimentacion", 0.5),
        ("mezcla", 0.5),
    ];

    #[test]
    fn mezcla_cero_es_identidad() {
        comprobar_identidad(
            &mut Phaser::default(),
            &[
                ("velocidad_hz", 1.0),
                ("profundidad", 1.0),
                ("realimentacion", 0.7),
                ("mezcla", 0.0),
            ],
        );
    }

    #[test]
    fn estable_con_ruido() {
        comprobar_estable(
            &mut Phaser::default(),
            &[
                ("velocidad_hz", 2.0),
                ("profundidad", 1.0),
                ("realimentacion", 0.9),
                ("mezcla", 1.0),
            ],
        );
    }

    #[test]
    fn sin_asignaciones_en_procesar() {
        comprobar_sin_asignaciones(&mut Phaser::default(), TIPICO);
    }

    #[test]
    fn phaser_modifica_la_senal() {
        let fs = 48_000.0;
        let mut pedal = Phaser::default();
        listo(&mut pedal, fs, TIPICO);
        let original = ruido(48_000, 9);
        let (mut izq, mut der) = (original.clone(), original.clone());
        procesar(&mut pedal, &mut izq, &mut der, 64);
        let diferencia: f32 = izq.iter().zip(&original).map(|(a, b)| (a - b).abs()).sum();
        assert!(diferencia > 100.0);
    }
}
