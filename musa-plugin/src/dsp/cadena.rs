use std::collections::BTreeMap;

use musa_catalogo::{PedalId, ORDEN_CADENA};

use super::chorus::Chorus;
use super::compresor::Compresor;
use super::delay::Delay;
use super::eq::Eq;
use super::overdrive::Overdrive;
use super::phaser::Phaser;
use super::puerta::Puerta;
use super::reverb::Reverb;
use super::{PasaTodo, Pedal, Rampa};

const FUNDIDO_MS: f32 = 20.0;

struct Eslabon {
    pedal: Box<dyn Pedal>,
    mezcla: Rampa,
}

pub struct Cadena {
    eslabones: Vec<Eslabon>,
    seco_izq: Vec<f32>,
    seco_der: Vec<f32>,
}

fn posicion(id: PedalId) -> usize {
    ORDEN_CADENA
        .iter()
        .position(|p| *p == id)
        .expect("todo PedalId está en ORDEN_CADENA")
}

impl Cadena {
    pub fn nueva() -> Self {
        Self {
            eslabones: ORDEN_CADENA
                .iter()
                .map(|_| Eslabon {
                    pedal: Box::new(PasaTodo),
                    mezcla: Rampa::nueva(FUNDIDO_MS),
                })
                .collect(),
            seco_izq: Vec::new(),
            seco_der: Vec::new(),
        }
    }

    pub fn completa() -> Self {
        let mut cadena = Self::nueva();
        cadena.poner(PedalId::Puerta, Box::new(Puerta::default()));
        cadena.poner(PedalId::Overdrive, Box::new(Overdrive::default()));
        cadena.poner(PedalId::Compresor, Box::new(Compresor::default()));
        cadena.poner(PedalId::Eq, Box::new(Eq::default()));
        cadena.poner(PedalId::Chorus, Box::new(Chorus::default()));
        cadena.poner(PedalId::Phaser, Box::new(Phaser::default()));
        cadena.poner(PedalId::Delay, Box::new(Delay::default()));
        cadena.poner(PedalId::Reverb, Box::new(Reverb::default()));
        cadena
    }

    pub fn poner(&mut self, id: PedalId, pedal: Box<dyn Pedal>) {
        self.eslabones[posicion(id)].pedal = pedal;
    }

    pub fn preparar(&mut self, frecuencia: f32, max_bloque: usize) {
        self.seco_izq = vec![0.0; max_bloque];
        self.seco_der = vec![0.0; max_bloque];
        for eslabon in &mut self.eslabones {
            eslabon.pedal.preparar(frecuencia, max_bloque);
            eslabon.mezcla.preparar(frecuencia);
        }
    }

    pub fn encender(&mut self, id: PedalId, encendido: bool) {
        let eslabon = &mut self.eslabones[posicion(id)];
        let apagado = eslabon.mezcla.estable() && eslabon.mezcla.actual() == 0.0;
        if encendido && apagado {
            eslabon.pedal.reiniciar();
        }
        eslabon.mezcla.objetivo(if encendido { 1.0 } else { 0.0 });
    }

    pub fn encendido(&self, id: PedalId) -> bool {
        let mezcla = &self.eslabones[posicion(id)].mezcla;
        !(mezcla.estable() && mezcla.actual() == 0.0)
    }

    pub fn aplicar(&mut self, id: PedalId, valores: &BTreeMap<String, f32>) {
        self.eslabones[posicion(id)].pedal.aplicar(valores);
    }

    pub fn reiniciar(&mut self) {
        for eslabon in &mut self.eslabones {
            eslabon.pedal.reiniciar();
        }
    }

    pub fn procesar(&mut self, izq: &mut [f32], der: &mut [f32]) {
        let n = izq.len().min(der.len());
        for inicio in (0..n).step_by(self.seco_izq.len().max(1)) {
            let fin = (inicio + self.seco_izq.len().max(1)).min(n);
            self.procesar_bloque(&mut izq[inicio..fin], &mut der[inicio..fin]);
        }
    }

    fn procesar_bloque(&mut self, izq: &mut [f32], der: &mut [f32]) {
        let n = izq.len();
        for eslabon in &mut self.eslabones {
            let mezcla = &mut eslabon.mezcla;
            if mezcla.estable() && mezcla.actual() == 0.0 {
                continue;
            }
            if mezcla.estable() && mezcla.actual() == 1.0 {
                eslabon.pedal.procesar(izq, der);
                continue;
            }
            let (seco_izq, seco_der) = (&mut self.seco_izq[..n], &mut self.seco_der[..n]);
            seco_izq.copy_from_slice(izq);
            seco_der.copy_from_slice(der);
            eslabon.pedal.procesar(izq, der);
            for i in 0..n {
                let m = mezcla.siguiente();
                izq[i] = seco_izq[i] + (izq[i] - seco_izq[i]) * m;
                der[i] = seco_der[i] + (der[i] - seco_der[i]) * m;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsp::pruebas::{salto_maximo, seno};

    const FS: f32 = 48_000.0;
    const BLOQUE: usize = 64;

    struct Invertir;

    impl Pedal for Invertir {
        fn preparar(&mut self, _frecuencia: f32, _max_bloque: usize) {}
        fn reiniciar(&mut self) {}
        fn aplicar(&mut self, _valores: &BTreeMap<String, f32>) {}
        fn procesar(&mut self, izq: &mut [f32], der: &mut [f32]) {
            izq.iter_mut().chain(der.iter_mut()).for_each(|x| *x = -*x);
        }
    }

    fn procesar_por_bloques(
        cadena: &mut Cadena,
        izq: &mut [f32],
        der: &mut [f32],
        al_bloque: impl Fn(usize, &mut Cadena),
    ) {
        for (n, (i, d)) in izq
            .chunks_mut(BLOQUE)
            .zip(der.chunks_mut(BLOQUE))
            .enumerate()
        {
            al_bloque(n, cadena);
            cadena.procesar(i, d);
        }
    }

    #[test]
    fn todo_apagado_es_identidad() {
        let original = seno(440.0, 0.5, 4800, FS);
        let (mut izq, mut der) = (original.clone(), original.clone());
        let mut cadena = Cadena::nueva();
        cadena.poner(PedalId::Chorus, Box::new(Invertir));
        cadena.preparar(FS, BLOQUE);
        procesar_por_bloques(&mut cadena, &mut izq, &mut der, |_, _| {});
        for (a, b) in izq.iter().zip(&original) {
            assert!((a - b).abs() < 1e-6);
        }
        assert_eq!(izq, der);
    }

    #[test]
    fn encender_no_produce_clicks() {
        let original = seno(440.0, 0.5, 9600, FS);
        let (mut izq, mut der) = (original.clone(), original.clone());
        let mut cadena = Cadena::nueva();
        cadena.poner(PedalId::Delay, Box::new(Invertir));
        cadena.preparar(FS, BLOQUE);
        procesar_por_bloques(&mut cadena, &mut izq, &mut der, |n, c| {
            if n == 37 {
                c.encender(PedalId::Delay, true);
            }
        });
        assert!(salto_maximo(&izq) < 0.05, "salto de {}", salto_maximo(&izq));
        let ultimo = original.len() - 1;
        assert!(
            (izq[ultimo] + original[ultimo]).abs() < 1e-6,
            "al final el pedal debe estar encendido"
        );
    }

    #[test]
    fn apagar_no_produce_clicks() {
        let original = seno(440.0, 0.5, 9600, FS);
        let (mut izq, mut der) = (original.clone(), original.clone());
        let mut cadena = Cadena::nueva();
        cadena.poner(PedalId::Reverb, Box::new(Invertir));
        cadena.preparar(FS, BLOQUE);
        cadena.encender(PedalId::Reverb, true);
        procesar_por_bloques(&mut cadena, &mut izq, &mut der, |n, c| {
            if n == 41 {
                c.encender(PedalId::Reverb, false);
            }
        });
        assert!(salto_maximo(&der[960..]) < 0.05);
        let ultimo = original.len() - 1;
        assert!((der[ultimo] - original[ultimo]).abs() < 1e-6);
    }

    #[test]
    #[ignore]
    fn cadena_completa_rendimiento() {
        use crate::dsp::pruebas::ruido;
        let mut cadena = Cadena::completa();
        cadena.preparar(FS, 512);
        for id in ORDEN_CADENA {
            cadena.encender(id, true);
        }
        let mut izq = ruido(480_000, 1);
        let mut der = ruido(480_000, 2);
        let inicio = std::time::Instant::now();
        for (i, d) in izq.chunks_mut(512).zip(der.chunks_mut(512)) {
            cadena.procesar(i, d);
        }
        let duracion = inicio.elapsed();
        println!("10 s de audio en {duracion:?}");
        assert!(duracion.as_secs_f32() < 1.5);
        assert!(izq.iter().all(|x| x.is_finite()));
    }
}
