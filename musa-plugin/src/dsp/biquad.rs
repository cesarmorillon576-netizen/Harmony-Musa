use std::f32::consts::TAU;

#[derive(Clone, Copy)]
pub struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    z1: f32,
    z2: f32,
}

impl Default for Biquad {
    fn default() -> Self {
        Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
            z1: 0.0,
            z2: 0.0,
        }
    }
}

pub enum Forma {
    PasaBajos(f32),
    EstanteBajo,
    Campana(f32),
    EstanteAlto,
}

impl Biquad {
    pub fn configurar(&mut self, forma: Forma, frecuencia: f32, fs: f32, ganancia_db: f32) {
        let a = 10f32.powf(ganancia_db / 40.0);
        let w0 = TAU * frecuencia / fs;
        let (sen, cos) = w0.sin_cos();
        let (b0, b1, b2, a0, a1, a2) = match forma {
            Forma::PasaBajos(q) => {
                let alfa = sen / (2.0 * q);
                let b1 = 1.0 - cos;
                (b1 / 2.0, b1, b1 / 2.0, 1.0 + alfa, -2.0 * cos, 1.0 - alfa)
            }
            Forma::Campana(q) => {
                let alfa = sen / (2.0 * q);
                (
                    1.0 + alfa * a,
                    -2.0 * cos,
                    1.0 - alfa * a,
                    1.0 + alfa / a,
                    -2.0 * cos,
                    1.0 - alfa / a,
                )
            }
            Forma::EstanteBajo => {
                let alfa = sen / 2.0 * 2f32.sqrt();
                let raiz = 2.0 * a.sqrt() * alfa;
                (
                    a * ((a + 1.0) - (a - 1.0) * cos + raiz),
                    2.0 * a * ((a - 1.0) - (a + 1.0) * cos),
                    a * ((a + 1.0) - (a - 1.0) * cos - raiz),
                    (a + 1.0) + (a - 1.0) * cos + raiz,
                    -2.0 * ((a - 1.0) + (a + 1.0) * cos),
                    (a + 1.0) + (a - 1.0) * cos - raiz,
                )
            }
            Forma::EstanteAlto => {
                let alfa = sen / 2.0 * 2f32.sqrt();
                let raiz = 2.0 * a.sqrt() * alfa;
                (
                    a * ((a + 1.0) + (a - 1.0) * cos + raiz),
                    -2.0 * a * ((a - 1.0) + (a + 1.0) * cos),
                    a * ((a + 1.0) + (a - 1.0) * cos - raiz),
                    (a + 1.0) - (a - 1.0) * cos + raiz,
                    2.0 * ((a - 1.0) - (a + 1.0) * cos),
                    (a + 1.0) - (a - 1.0) * cos - raiz,
                )
            }
        };
        self.b0 = b0 / a0;
        self.b1 = b1 / a0;
        self.b2 = b2 / a0;
        self.a1 = a1 / a0;
        self.a2 = a2 / a0;
    }

    pub fn reiniciar(&mut self) {
        self.z1 = 0.0;
        self.z2 = 0.0;
    }

    pub fn procesar(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.z1;
        self.z1 = self.b1 * x - self.a1 * y + self.z2;
        self.z2 = self.b2 * x - self.a2 * y;
        y
    }
}
