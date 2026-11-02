pub struct Rampa {
    ms: f32,
    actual: f32,
    destino: f32,
    paso: f32,
    pasos_por_unidad: f32,
}

impl Rampa {
    pub fn nueva(ms: f32) -> Self {
        Self {
            ms,
            actual: 0.0,
            destino: 0.0,
            paso: 0.0,
            pasos_por_unidad: 1.0,
        }
    }

    pub fn preparar(&mut self, frecuencia: f32) {
        self.pasos_por_unidad = (self.ms / 1000.0 * frecuencia).max(1.0);
        self.fijar(self.destino);
    }

    pub fn fijar(&mut self, valor: f32) {
        self.actual = valor;
        self.destino = valor;
        self.paso = 0.0;
    }

    pub fn saltar(&mut self) {
        self.fijar(self.destino);
    }

    pub fn objetivo(&mut self, valor: f32) {
        if valor == self.destino {
            return;
        }
        self.destino = valor;
        self.paso = (valor - self.actual) / self.pasos_por_unidad;
    }

    pub fn siguiente(&mut self) -> f32 {
        if self.paso != 0.0 {
            self.actual += self.paso;
            let llego = if self.paso > 0.0 {
                self.actual >= self.destino
            } else {
                self.actual <= self.destino
            };
            if llego {
                self.fijar(self.destino);
            }
        }
        self.actual
    }

    pub fn actual(&self) -> f32 {
        self.actual
    }

    pub fn estable(&self) -> bool {
        self.paso == 0.0
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rampa_llega_al_objetivo() {
        let mut rampa = Rampa::nueva(20.0);
        rampa.preparar(48_000.0);
        rampa.fijar(0.0);
        rampa.objetivo(1.0);
        let mut muestras = 0;
        while rampa.siguiente() < 1.0 {
            muestras += 1;
            assert!(muestras <= 960, "no llegó en 960 muestras");
        }
        assert!(rampa.estable());
        assert_eq!(rampa.siguiente(), 1.0);
    }

    #[test]
    fn rampa_es_monotona() {
        let mut rampa = Rampa::nueva(20.0);
        rampa.preparar(48_000.0);
        rampa.fijar(1.0);
        rampa.objetivo(0.0);
        let mut anterior = 1.0;
        for _ in 0..1000 {
            let v = rampa.siguiente();
            assert!(v <= anterior && v >= 0.0);
            anterior = v;
        }
        assert_eq!(anterior, 0.0);
    }
}
