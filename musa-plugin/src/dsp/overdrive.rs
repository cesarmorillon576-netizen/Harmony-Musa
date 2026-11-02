use std::collections::BTreeMap;
use std::f32::consts::TAU;

use super::biquad::{Biquad, Forma};
use super::{db_a_ganancia, valor, Pedal, Rampa};

const CORTE_GRAVES_HZ: f32 = 90.0;
const BOCINA_MINIMA_HZ: f32 = 2_000.0;
const BOCINA_MAXIMA_HZ: f32 = 7_000.0;
const Q_BOCINA: f32 = 0.7071;
const ASIMETRIA: f32 = 0.12;
const GANANCIA_INTERNA_DB: f32 = 12.0;
const SEGUNDA_ETAPA: f32 = 1.5;
const NIVEL_SALIDA: f32 = 0.35;

pub struct Overdrive {
    fs: f32,
    ganancia: Rampa,
    mezcla: Rampa,
    tono: f32,
    pasa_altos: [f32; 2],
    previo: [f32; 2],
    coef_pasa_altos: f32,
    bocina: [[Biquad; 2]; 2],
}

impl Default for Overdrive {
    fn default() -> Self {
        Self {
            fs: 48_000.0,
            ganancia: Rampa::nueva(40.0),
            mezcla: Rampa::nueva(30.0),
            tono: 0.5,
            pasa_altos: [0.0; 2],
            previo: [0.0; 2],
            coef_pasa_altos: 0.0,
            bocina: [[Biquad::default(); 2]; 2],
        }
    }
}

impl Overdrive {
    fn configurar_bocina(&mut self) {
        let corte = BOCINA_MINIMA_HZ
            * (BOCINA_MAXIMA_HZ / BOCINA_MINIMA_HZ).powf(self.tono.clamp(0.0, 1.0));
        let corte = corte.min(self.fs * 0.45);
        for canal in &mut self.bocina {
            for filtro in canal.iter_mut() {
                filtro.configurar(Forma::PasaBajos(Q_BOCINA), corte, self.fs, 0.0);
            }
        }
        self.coef_pasa_altos = 1.0 / (1.0 + TAU * CORTE_GRAVES_HZ / self.fs);
    }
}

impl Pedal for Overdrive {
    fn preparar(&mut self, frecuencia: f32, _max_bloque: usize) {
        self.fs = frecuencia;
        self.ganancia.preparar(frecuencia);
        self.mezcla.preparar(frecuencia);
        self.configurar_bocina();
    }

    fn reiniciar(&mut self) {
        self.pasa_altos = [0.0; 2];
        self.previo = [0.0; 2];
        self.bocina.iter_mut().flatten().for_each(Biquad::reiniciar);
        self.ganancia.saltar();
        self.mezcla.saltar();
    }

    fn aplicar(&mut self, valores: &BTreeMap<String, f32>) {
        self.ganancia.objetivo(db_a_ganancia(
            valor(valores, "ganancia_db", 18.0).max(0.0) + GANANCIA_INTERNA_DB,
        ));
        self.mezcla
            .objetivo(valor(valores, "mezcla", 1.0).clamp(0.0, 1.0));
        self.tono = valor(valores, "tono", 0.5);
        self.configurar_bocina();
    }

    fn procesar(&mut self, izq: &mut [f32], der: &mut [f32]) {
        let desplazamiento = ASIMETRIA.tanh();
        for (l, r) in izq.iter_mut().zip(der.iter_mut()) {
            let ganancia = self.ganancia.siguiente();
            let mezcla = self.mezcla.siguiente();
            for (canal, muestra) in [l, r].into_iter().enumerate() {
                let x = *muestra;
                let filtrado =
                    self.coef_pasa_altos * (self.pasa_altos[canal] + x - self.previo[canal]);
                self.pasa_altos[canal] = filtrado;
                self.previo[canal] = x;
                let primera = (ganancia * filtrado + ASIMETRIA).tanh() - desplazamiento;
                let segunda = (SEGUNDA_ETAPA * primera).tanh();
                let mojado = self.bocina[canal]
                    .iter_mut()
                    .fold(segunda * NIVEL_SALIDA, |s, f| f.procesar(s));
                *muestra = x + (mojado - x) * mezcla;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsp::pruebas::*;

    const FS: f32 = 48_000.0;
    const TIPICO: &[(&str, f32)] = &[("ganancia_db", 24.0), ("tono", 0.5), ("mezcla", 1.0)];

    fn magnitud(senal: &[f32], frecuencia: f32) -> f32 {
        let w = std::f32::consts::TAU * frecuencia / FS;
        let (re, im) = senal
            .iter()
            .enumerate()
            .fold((0.0, 0.0), |(re, im), (n, x)| {
                (re + x * (w * n as f32).cos(), im - x * (w * n as f32).sin())
            });
        (re * re + im * im).sqrt()
    }

    fn distorsion(senal: &[f32], fundamental: f32) -> f32 {
        let base = magnitud(senal, fundamental);
        let armonicos: f32 = (2..=6)
            .map(|k| magnitud(senal, fundamental * k as f32).powi(2))
            .sum();
        armonicos.sqrt() / base
    }

    fn rms(senal: &[f32]) -> f32 {
        (energia(senal) / senal.len() as f32).sqrt()
    }

    #[test]
    fn mezcla_cero_es_identidad() {
        comprobar_identidad(
            &mut Overdrive::default(),
            &[("ganancia_db", 30.0), ("tono", 0.5), ("mezcla", 0.0)],
        );
    }

    #[test]
    fn estable_con_ruido() {
        comprobar_estable(
            &mut Overdrive::default(),
            &[("ganancia_db", 40.0), ("tono", 1.0), ("mezcla", 1.0)],
        );
    }

    #[test]
    fn sin_asignaciones_en_procesar() {
        comprobar_sin_asignaciones(&mut Overdrive::default(), TIPICO);
    }

    #[test]
    fn overdrive_satura() {
        let mut pedal = Overdrive::default();
        listo(
            &mut pedal,
            FS,
            &[("ganancia_db", 30.0), ("tono", 0.7), ("mezcla", 1.0)],
        );
        let original = seno(196.0, 0.5, 24_000, FS);
        let (mut izq, mut der) = (original.clone(), original.clone());
        procesar(&mut pedal, &mut izq, &mut der, 64);
        let cola = &izq[12_000..];
        let factor_cresta = pico(cola) / rms(cola);
        assert!(factor_cresta < 1.3, "no satura: cresta {factor_cresta}");
        assert!(pico(cola) < 1.0);
    }

    #[test]
    fn nivel_parecido_al_original() {
        let mut pedal = Overdrive::default();
        listo(&mut pedal, FS, TIPICO);
        let original = seno(196.0, 0.3, 24_000, FS);
        let (mut izq, mut der) = (original.clone(), original.clone());
        procesar(&mut pedal, &mut izq, &mut der, 64);
        let relacion_db = 20.0 * (rms(&izq[12_000..]) / rms(&original[12_000..])).log10();
        assert!(
            relacion_db.abs() < 6.0,
            "cambia el volumen {relacion_db} dB"
        );
    }

    #[test]
    fn filtro_de_bocina_corta_agudos_asperos() {
        let mut pedal = Overdrive::default();
        listo(
            &mut pedal,
            FS,
            &[("ganancia_db", 0.0), ("tono", 0.5), ("mezcla", 1.0)],
        );
        let original = seno(12_000.0, 0.01, 24_000, FS);
        let (mut izq, mut der) = (original.clone(), original.clone());
        procesar(&mut pedal, &mut izq, &mut der, 64);
        assert!(
            pico(&izq[12_000..]) < 0.01 * 0.25,
            "12 kHz pasa casi completo"
        );
    }

    #[test]
    fn se_nota_con_guitarra_directa() {
        let mut pedal = Overdrive::default();
        listo(
            &mut pedal,
            FS,
            &[("ganancia_db", 18.0), ("tono", 0.6), ("mezcla", 1.0)],
        );
        let original = seno(196.0, 0.1, 24_000, FS);
        let (mut izq, mut der) = (original.clone(), original.clone());
        procesar(&mut pedal, &mut izq, &mut der, 64);
        let cola = &izq[12_000..];
        let thd = distorsion(cola, 196.0);
        assert!(thd > 0.15, "casi no distorsiona: THD {thd}");
        let relacion_db = 20.0 * (rms(cola) / rms(&original[12_000..])).log10();
        assert!(
            relacion_db > 3.0,
            "queda más bajo que el limpio: {relacion_db} dB"
        );
    }
}
