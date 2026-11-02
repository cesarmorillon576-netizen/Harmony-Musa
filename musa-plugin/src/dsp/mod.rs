use std::collections::BTreeMap;

pub mod biquad;
pub mod cadena;
pub mod chorus;
pub mod compresor;
pub mod delay;
pub mod eq;
pub mod overdrive;
pub mod phaser;
pub mod puerta;
pub mod rampa;
pub mod reverb;
pub mod tremolo;

pub use cadena::Cadena;
pub use rampa::Rampa;

pub trait Pedal: Send {
    fn preparar(&mut self, frecuencia: f32, max_bloque: usize);
    fn reiniciar(&mut self);
    fn aplicar(&mut self, valores: &BTreeMap<String, f32>);
    fn procesar(&mut self, izq: &mut [f32], der: &mut [f32]);
}

pub fn valor(valores: &BTreeMap<String, f32>, clave: &str, defecto: f32) -> f32 {
    valores.get(clave).copied().unwrap_or(defecto)
}

pub fn db_a_ganancia(db: f32) -> f32 {
    10f32.powf(db / 20.0)
}

pub struct PasaTodo;

impl Pedal for PasaTodo {
    fn preparar(&mut self, _frecuencia: f32, _max_bloque: usize) {}
    fn reiniciar(&mut self) {}
    fn aplicar(&mut self, _valores: &BTreeMap<String, f32>) {}
    fn procesar(&mut self, _izq: &mut [f32], _der: &mut [f32]) {}
}

#[cfg(test)]
pub mod pruebas {
    use std::collections::BTreeMap;

    use super::Pedal;

    pub fn seno(frecuencia: f32, amplitud: f32, muestras: usize, fs: f32) -> Vec<f32> {
        (0..muestras)
            .map(|n| amplitud * (std::f32::consts::TAU * frecuencia * n as f32 / fs).sin())
            .collect()
    }

    pub fn ruido(muestras: usize, semilla: u32) -> Vec<f32> {
        let mut estado = semilla.max(1);
        (0..muestras)
            .map(|_| {
                estado ^= estado << 13;
                estado ^= estado >> 17;
                estado ^= estado << 5;
                estado as f32 / u32::MAX as f32 * 2.0 - 1.0
            })
            .collect()
    }

    pub fn impulso(muestras: usize) -> Vec<f32> {
        let mut senal = vec![0.0; muestras];
        senal[0] = 1.0;
        senal
    }

    pub fn salto_maximo(senal: &[f32]) -> f32 {
        senal
            .windows(2)
            .map(|w| (w[1] - w[0]).abs())
            .fold(0.0, f32::max)
    }

    pub fn pico(senal: &[f32]) -> f32 {
        senal.iter().fold(0.0, |m, x| m.max(x.abs()))
    }

    pub fn energia(senal: &[f32]) -> f32 {
        senal.iter().map(|x| x * x).sum()
    }

    pub fn valores(pares: &[(&str, f32)]) -> BTreeMap<String, f32> {
        pares.iter().map(|(k, v)| (k.to_string(), *v)).collect()
    }

    pub fn procesar(pedal: &mut dyn Pedal, izq: &mut [f32], der: &mut [f32], bloque: usize) {
        for (i, d) in izq.chunks_mut(bloque).zip(der.chunks_mut(bloque)) {
            pedal.procesar(i, d);
        }
    }

    pub fn listo(pedal: &mut dyn Pedal, fs: f32, pares: &[(&str, f32)]) {
        pedal.preparar(fs, 2048);
        pedal.aplicar(&valores(pares));
        pedal.reiniciar();
    }

    pub fn comprobar_identidad(pedal: &mut dyn Pedal, pares: &[(&str, f32)]) {
        let fs = 48_000.0;
        listo(pedal, fs, pares);
        let original = seno(440.0, 0.5, 4800, fs);
        let (mut izq, mut der) = (original.clone(), original.clone());
        procesar(pedal, &mut izq, &mut der, 64);
        for (a, b) in izq.iter().chain(&der).zip(original.iter().chain(&original)) {
            assert!((a - b).abs() < 1e-4, "{a} != {b}");
        }
    }

    pub fn comprobar_estable(pedal: &mut dyn Pedal, pares: &[(&str, f32)]) {
        let fs = 48_000.0;
        listo(pedal, fs, pares);
        let mut izq = ruido(48_000, 7);
        let mut der = ruido(48_000, 11);
        procesar(pedal, &mut izq, &mut der, 256);
        for x in izq.iter().chain(&der) {
            assert!(
                x.is_finite() && x.abs() < 4.0,
                "muestra fuera de rango: {x}"
            );
        }
    }

    pub fn comprobar_sin_asignaciones(pedal: &mut dyn Pedal, pares: &[(&str, f32)]) {
        for fs in [44_100.0, 48_000.0, 96_000.0] {
            listo(pedal, fs, pares);
            for bloque in [64, 512, 2048] {
                let mut izq = ruido(bloque, 3);
                let mut der = ruido(bloque, 5);
                assert_no_alloc::assert_no_alloc(|| pedal.procesar(&mut izq, &mut der));
            }
        }
    }
}
