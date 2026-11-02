use std::collections::BTreeMap;

use super::biquad::{Biquad, Forma};
use super::{valor, Pedal};

const FRECUENCIA_GRAVES: f32 = 200.0;
const FRECUENCIA_MEDIOS: f32 = 800.0;
const Q_MEDIOS: f32 = 0.7;
const FRECUENCIA_AGUDOS: f32 = 4_000.0;
const BLOQUE_CONTROL: usize = 32;

#[derive(Default)]
pub struct Eq {
    fs: f32,
    filtros: [[Biquad; 3]; 2],
    graves: f32,
    medios: f32,
    agudos: f32,
    actuales: [f32; 3],
}

impl Eq {
    fn configurar(&mut self, ganancias: [f32; 3]) {
        self.actuales = ganancias;
        for canal in &mut self.filtros {
            canal[0].configurar(Forma::EstanteBajo, FRECUENCIA_GRAVES, self.fs, ganancias[0]);
            canal[1].configurar(
                Forma::Campana(Q_MEDIOS),
                FRECUENCIA_MEDIOS,
                self.fs,
                ganancias[1],
            );
            canal[2].configurar(Forma::EstanteAlto, FRECUENCIA_AGUDOS, self.fs, ganancias[2]);
        }
    }

    fn objetivos(&self) -> [f32; 3] {
        [self.graves, self.medios, self.agudos]
    }
}

impl Pedal for Eq {
    fn preparar(&mut self, frecuencia: f32, _max_bloque: usize) {
        self.fs = frecuencia;
        self.configurar(self.objetivos());
    }

    fn reiniciar(&mut self) {
        self.configurar(self.objetivos());
        self.filtros
            .iter_mut()
            .flatten()
            .for_each(Biquad::reiniciar);
    }

    fn aplicar(&mut self, valores: &BTreeMap<String, f32>) {
        self.graves = valor(valores, "graves_db", 0.0);
        self.medios = valor(valores, "medios_db", 0.0);
        self.agudos = valor(valores, "agudos_db", 0.0);
    }

    fn procesar(&mut self, izq: &mut [f32], der: &mut [f32]) {
        for (bloque_izq, bloque_der) in izq
            .chunks_mut(BLOQUE_CONTROL)
            .zip(der.chunks_mut(BLOQUE_CONTROL))
        {
            let objetivos = self.objetivos();
            if objetivos != self.actuales {
                let siguiente = std::array::from_fn(|i| {
                    self.actuales[i] + (objetivos[i] - self.actuales[i]).clamp(-0.5, 0.5)
                });
                self.configurar(siguiente);
            }
            for (canal, muestras) in [bloque_izq, bloque_der].into_iter().enumerate() {
                for x in muestras.iter_mut() {
                    *x = self.filtros[canal]
                        .iter_mut()
                        .fold(*x, |s, f| f.procesar(s));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsp::pruebas::*;

    #[test]
    fn plano_es_identidad() {
        comprobar_identidad(
            &mut Eq::default(),
            &[("graves_db", 0.0), ("medios_db", 0.0), ("agudos_db", 0.0)],
        );
    }

    #[test]
    fn estable_con_ruido() {
        comprobar_estable(
            &mut Eq::default(),
            &[("graves_db", 9.0), ("medios_db", -6.0), ("agudos_db", 9.0)],
        );
    }

    #[test]
    fn sin_asignaciones_en_procesar() {
        comprobar_sin_asignaciones(
            &mut Eq::default(),
            &[("graves_db", 6.0), ("medios_db", 3.0), ("agudos_db", -3.0)],
        );
    }

    #[test]
    fn eq_realza_graves() {
        let fs = 48_000.0;
        let mut pedal = Eq::default();
        listo(
            &mut pedal,
            fs,
            &[("graves_db", 12.0), ("medios_db", 0.0), ("agudos_db", 0.0)],
        );
        let original = seno(80.0, 0.1, 48_000, fs);
        let (mut izq, mut der) = (original.clone(), original.clone());
        procesar(&mut pedal, &mut izq, &mut der, 512);
        let ganancia_db = 20.0 * (pico(&izq[24_000..]) / pico(&original[24_000..])).log10();
        assert!(ganancia_db >= 9.0, "solo {ganancia_db} dB");
    }
}
