use std::collections::BTreeMap;

use super::Pedal;

#[derive(Default)]
pub struct Tremolo;

impl Pedal for Tremolo {
    fn preparar(&mut self, _frecuencia: f32, _max_bloque: usize) {
        todo!("Tarea 6")
    }
    fn reiniciar(&mut self) {
        todo!("Tarea 6")
    }
    fn aplicar(&mut self, _valores: &BTreeMap<String, f32>) {
        todo!("Tarea 6")
    }
    fn procesar(&mut self, _izq: &mut [f32], _der: &mut [f32]) {
        todo!("Tarea 6")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsp::pruebas::*;

    const FS: f32 = 48_000.0;

    fn constante(valor: f32, pares: &[(&str, f32)]) -> Vec<f32> {
        let mut pedal = Tremolo::default();
        listo(&mut pedal, FS, pares);
        let mut izq = vec![valor; 48_000];
        let mut der = vec![valor; 48_000];
        procesar(&mut pedal, &mut izq, &mut der, 64);
        assert_eq!(izq, der);
        izq
    }

    fn minimos(senal: &[f32]) -> Vec<usize> {
        (1..senal.len() - 1)
            .filter(|&i| senal[i] < senal[i - 1] && senal[i] <= senal[i + 1])
            .collect()
    }

    #[test]
    fn profundidad_cero_es_identidad() {
        comprobar_identidad(
            &mut Tremolo::default(),
            &[("velocidad_hz", 5.0), ("profundidad", 0.0), ("forma", 0.0)],
        );
    }

    #[test]
    fn periodo_correcto() {
        let salida = constante(
            1.0,
            &[("velocidad_hz", 4.0), ("profundidad", 0.8), ("forma", 0.0)],
        );
        let minimos = minimos(&salida);
        assert!(minimos.len() >= 3, "no se encontraron mínimos");
        for par in minimos.windows(2) {
            let distancia = par[1] - par[0];
            assert!(
                distancia.abs_diff(12_000) <= 2,
                "mínimos separados {distancia} muestras"
            );
        }
    }

    #[test]
    fn profundidad_total_llega_a_cero() {
        let salida = constante(
            1.0,
            &[("velocidad_hz", 4.0), ("profundidad", 1.0), ("forma", 0.0)],
        );
        let minimo = salida.iter().cloned().fold(f32::MAX, f32::min);
        assert!(minimo < 0.01, "la ganancia mínima es {minimo}");
    }

    #[test]
    fn cuadrada_sin_clicks() {
        let salida = constante(
            0.5,
            &[("velocidad_hz", 6.0), ("profundidad", 1.0), ("forma", 1.0)],
        );
        assert!(salida_varia(&salida), "la onda cuadrada no modula");
        assert!(
            salto_maximo(&salida) < 0.05,
            "salto de {}",
            salto_maximo(&salida)
        );
    }

    fn salida_varia(senal: &[f32]) -> bool {
        let (min, max) = senal
            .iter()
            .fold((f32::MAX, f32::MIN), |(a, b), &x| (a.min(x), b.max(x)));
        max - min > 0.4
    }

    #[test]
    fn estable_con_ruido() {
        comprobar_estable(
            &mut Tremolo::default(),
            &[("velocidad_hz", 8.0), ("profundidad", 1.0), ("forma", 1.0)],
        );
    }

    #[test]
    fn sin_asignaciones_en_procesar() {
        comprobar_sin_asignaciones(
            &mut Tremolo::default(),
            &[("velocidad_hz", 5.0), ("profundidad", 0.7), ("forma", 0.0)],
        );
    }
}
