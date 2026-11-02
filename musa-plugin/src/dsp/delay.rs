use std::collections::BTreeMap;

use super::chorus::leer_interpolado;
use super::{valor, Pedal, Rampa};

const MAXIMO_MS: f32 = 1_500.0;

pub struct Delay {
    fs: f32,
    lineas: [Vec<f32>; 2],
    escritura: usize,
    tiempo: Rampa,
    realimentacion: Rampa,
    mezcla: Rampa,
    ping_pong: bool,
}

impl Default for Delay {
    fn default() -> Self {
        Self {
            fs: 48_000.0,
            lineas: [Vec::new(), Vec::new()],
            escritura: 0,
            tiempo: Rampa::nueva(150.0),
            realimentacion: Rampa::nueva(30.0),
            mezcla: Rampa::nueva(30.0),
            ping_pong: false,
        }
    }
}

impl Pedal for Delay {
    fn preparar(&mut self, frecuencia: f32, _max_bloque: usize) {
        self.fs = frecuencia;
        let largo = (MAXIMO_MS / 1000.0 * frecuencia) as usize + 4;
        self.lineas = [vec![0.0; largo], vec![0.0; largo]];
        self.escritura = 0;
        for rampa in [&mut self.tiempo, &mut self.realimentacion, &mut self.mezcla] {
            rampa.preparar(frecuencia);
        }
    }

    fn reiniciar(&mut self) {
        self.lineas.iter_mut().for_each(|l| l.fill(0.0));
        for rampa in [&mut self.tiempo, &mut self.realimentacion, &mut self.mezcla] {
            rampa.saltar();
        }
    }

    fn aplicar(&mut self, valores: &BTreeMap<String, f32>) {
        self.tiempo
            .objetivo(valor(valores, "tiempo_ms", 350.0).clamp(1.0, MAXIMO_MS));
        self.realimentacion
            .objetivo(valor(valores, "realimentacion", 0.4).clamp(0.0, 0.95));
        self.mezcla
            .objetivo(valor(valores, "mezcla", 0.35).clamp(0.0, 1.0));
        self.ping_pong = valor(valores, "ping_pong", 0.0) >= 0.5;
    }

    fn procesar(&mut self, izq: &mut [f32], der: &mut [f32]) {
        let largo = self.lineas[0].len();
        let ms_a_muestras = self.fs / 1000.0;
        for (l, r) in izq.iter_mut().zip(der.iter_mut()) {
            let retardo = self.tiempo.siguiente() * ms_a_muestras;
            let realimentacion = self.realimentacion.siguiente();
            let mezcla = self.mezcla.siguiente();
            let eco_izq = leer_interpolado(&self.lineas[0], self.escritura, retardo);
            let eco_der = leer_interpolado(&self.lineas[1], self.escritura, retardo);
            if self.ping_pong {
                self.lineas[0][self.escritura] = (*l + *r) * 0.5 + eco_der * realimentacion;
                self.lineas[1][self.escritura] = eco_izq * realimentacion;
            } else {
                self.lineas[0][self.escritura] = *l + eco_izq * realimentacion;
                self.lineas[1][self.escritura] = *r + eco_der * realimentacion;
            }
            *l += eco_izq * mezcla;
            *r += eco_der * mezcla;
            self.escritura = (self.escritura + 1) % largo;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsp::pruebas::*;

    const TIPICO: &[(&str, f32)] = &[
        ("tiempo_ms", 350.0),
        ("realimentacion", 0.6),
        ("mezcla", 0.4),
        ("ping_pong", 1.0),
    ];

    fn primer_eco(senal: &[f32], desde: usize) -> usize {
        desde
            + senal[desde..]
                .iter()
                .position(|x| x.abs() > 1e-3)
                .expect("no hay eco")
    }

    #[test]
    fn mezcla_cero_es_identidad() {
        comprobar_identidad(
            &mut Delay::default(),
            &[
                ("tiempo_ms", 300.0),
                ("realimentacion", 0.5),
                ("mezcla", 0.0),
                ("ping_pong", 0.0),
            ],
        );
    }

    #[test]
    fn estable_con_ruido() {
        comprobar_estable(&mut Delay::default(), TIPICO);
    }

    #[test]
    fn sin_asignaciones_en_procesar() {
        comprobar_sin_asignaciones(&mut Delay::default(), TIPICO);
    }

    #[test]
    fn delay_impulso_en_su_tiempo() {
        for (fs, esperado) in [(48_000.0, 12_000usize), (44_100.0, 11_025)] {
            let mut pedal = Delay::default();
            listo(
                &mut pedal,
                fs,
                &[
                    ("tiempo_ms", 250.0),
                    ("realimentacion", 0.0),
                    ("mezcla", 0.5),
                    ("ping_pong", 0.0),
                ],
            );
            let mut izq = impulso(fs as usize);
            let mut der = impulso(fs as usize);
            procesar(&mut pedal, &mut izq, &mut der, 512);
            let eco = primer_eco(&izq, 1);
            assert!(
                eco.abs_diff(esperado) <= 1,
                "eco en {eco}, esperado {esperado}"
            );
        }
    }

    #[test]
    fn delay_ping_pong_alterna() {
        let fs = 48_000.0;
        let mut pedal = Delay::default();
        listo(
            &mut pedal,
            fs,
            &[
                ("tiempo_ms", 250.0),
                ("realimentacion", 0.6),
                ("mezcla", 0.5),
                ("ping_pong", 1.0),
            ],
        );
        let mut izq = impulso(36_000);
        let mut der = impulso(36_000);
        procesar(&mut pedal, &mut izq, &mut der, 512);
        let ventana = |s: &[f32], centro: usize| pico(&s[centro - 5..centro + 5]);
        assert!(ventana(&izq, 12_000) > 0.05 && ventana(&der, 12_000) < 1e-3);
        assert!(ventana(&der, 24_000) > 0.02 && ventana(&izq, 24_000) < 1e-3);
    }
}
