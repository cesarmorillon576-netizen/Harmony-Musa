use std::collections::BTreeMap;
use std::f32::consts::{FRAC_PI_2, TAU};

use super::{valor, Pedal, Rampa};

const RETARDO_MINIMO_MS: f32 = 7.0;
const BARRIDO_MS: f32 = 18.0;

pub struct Chorus {
    fs: f32,
    lineas: [Vec<f32>; 2],
    escritura: usize,
    fase: f32,
    velocidad: f32,
    profundidad: Rampa,
    mezcla: Rampa,
}

impl Default for Chorus {
    fn default() -> Self {
        Self {
            fs: 48_000.0,
            lineas: [Vec::new(), Vec::new()],
            escritura: 0,
            fase: 0.0,
            velocidad: 0.8,
            profundidad: Rampa::nueva(30.0),
            mezcla: Rampa::nueva(30.0),
        }
    }
}

pub fn leer_interpolado(linea: &[f32], escritura: usize, retardo: f32) -> f32 {
    let largo = linea.len();
    let posicion = escritura as f32 + largo as f32 - retardo;
    let entero = posicion.floor();
    let fraccion = posicion - entero;
    let i = entero as usize % largo;
    let a = linea[i];
    let b = linea[(i + 1) % largo];
    a + (b - a) * fraccion
}

impl Pedal for Chorus {
    fn preparar(&mut self, frecuencia: f32, _max_bloque: usize) {
        self.fs = frecuencia;
        let largo = ((RETARDO_MINIMO_MS + BARRIDO_MS) / 1000.0 * frecuencia) as usize + 4;
        self.lineas = [vec![0.0; largo], vec![0.0; largo]];
        self.profundidad.preparar(frecuencia);
        self.mezcla.preparar(frecuencia);
        self.escritura = 0;
    }

    fn reiniciar(&mut self) {
        self.lineas.iter_mut().for_each(|l| l.fill(0.0));
        self.fase = 0.0;
        self.profundidad.saltar();
        self.mezcla.saltar();
    }

    fn aplicar(&mut self, valores: &BTreeMap<String, f32>) {
        self.velocidad = valor(valores, "velocidad_hz", 0.8);
        self.profundidad
            .objetivo(valor(valores, "profundidad", 0.5).clamp(0.0, 1.0));
        self.mezcla
            .objetivo(valor(valores, "mezcla", 0.5).clamp(0.0, 1.0));
    }

    fn procesar(&mut self, izq: &mut [f32], der: &mut [f32]) {
        let ms_a_muestras = self.fs / 1000.0;
        let largo = self.lineas[0].len();
        for (l, r) in izq.iter_mut().zip(der.iter_mut()) {
            let profundidad = self.profundidad.siguiente();
            let mezcla = self.mezcla.siguiente();
            self.lineas[0][self.escritura] = *l;
            self.lineas[1][self.escritura] = *r;
            for (canal, muestra) in [l, r].into_iter().enumerate() {
                let lfo = 0.5 + 0.5 * (self.fase * TAU + canal as f32 * FRAC_PI_2).sin();
                let retardo = (RETARDO_MINIMO_MS + BARRIDO_MS * profundidad * lfo) * ms_a_muestras;
                let mojado = leer_interpolado(&self.lineas[canal], self.escritura, retardo);
                *muestra += (mojado - *muestra) * mezcla;
            }
            self.escritura = (self.escritura + 1) % largo;
            self.fase = (self.fase + self.velocidad / self.fs).fract();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsp::pruebas::*;

    const TIPICO: &[(&str, f32)] = &[("velocidad_hz", 0.8), ("profundidad", 0.6), ("mezcla", 0.5)];

    #[test]
    fn mezcla_cero_es_identidad() {
        comprobar_identidad(
            &mut Chorus::default(),
            &[("velocidad_hz", 1.0), ("profundidad", 1.0), ("mezcla", 0.0)],
        );
    }

    #[test]
    fn estable_con_ruido() {
        comprobar_estable(&mut Chorus::default(), TIPICO);
    }

    #[test]
    fn sin_asignaciones_en_procesar() {
        comprobar_sin_asignaciones(&mut Chorus::default(), TIPICO);
    }

    #[test]
    fn chorus_modifica_la_senal() {
        let fs = 48_000.0;
        let mut pedal = Chorus::default();
        listo(&mut pedal, fs, TIPICO);
        let original = seno(440.0, 0.5, 9600, fs);
        let (mut izq, mut der) = (original.clone(), original.clone());
        procesar(&mut pedal, &mut izq, &mut der, 64);
        let diferencia: f32 = izq.iter().zip(&original).map(|(a, b)| (a - b).abs()).sum();
        assert!(diferencia > 10.0);
        assert_ne!(izq, der, "el chorus debe abrir el estéreo");
    }
}
