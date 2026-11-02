use std::sync::{Arc, RwLock};

use musa_catalogo::{PedalId, Preset, PEDALES};
use nih_plug::prelude::*;
use nih_plug_egui::EguiState;

use crate::gui;
use crate::presets::{etiqueta_preset, MAXIMO_PRESETS};

#[derive(Enum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Modo {
    #[name = "Según preset"]
    SegunPreset,
    #[name = "Encendido"]
    Encendido,
    #[name = "Apagado"]
    Apagado,
}

#[derive(Params)]
pub struct MusaParams {
    #[persist = "editor-state"]
    pub editor_state: Arc<EguiState>,

    #[persist = "presets"]
    pub presets: Arc<RwLock<Vec<Preset>>>,

    #[id = "preset"]
    pub preset: IntParam,

    #[id = "modo_puerta"]
    pub modo_puerta: EnumParam<Modo>,
    #[id = "modo_overdrive"]
    pub modo_overdrive: EnumParam<Modo>,
    #[id = "modo_compresor"]
    pub modo_compresor: EnumParam<Modo>,
    #[id = "modo_eq"]
    pub modo_eq: EnumParam<Modo>,
    #[id = "modo_chorus"]
    pub modo_chorus: EnumParam<Modo>,
    #[id = "modo_phaser"]
    pub modo_phaser: EnumParam<Modo>,
    #[id = "modo_tremolo"]
    pub modo_tremolo: EnumParam<Modo>,
    #[id = "modo_delay"]
    pub modo_delay: EnumParam<Modo>,
    #[id = "modo_reverb"]
    pub modo_reverb: EnumParam<Modo>,
}

impl Default for MusaParams {
    fn default() -> Self {
        let modo = |nombre: &str| EnumParam::new(nombre, Modo::SegunPreset);
        let presets = Arc::new(RwLock::new(Vec::new()));
        let para_etiqueta = presets.clone();
        Self {
            editor_state: EguiState::from_size(gui::ANCHO, gui::ALTO),
            presets,
            preset: IntParam::new(
                "Preset",
                0,
                IntRange::Linear {
                    min: 0,
                    max: MAXIMO_PRESETS as i32 - 1,
                },
            )
            .with_value_to_string(Arc::new(move |indice| {
                match para_etiqueta.try_read() {
                    Ok(presets) => etiqueta_preset(&presets, indice),
                    Err(_) => (indice + 1).to_string(),
                }
            })),
            modo_puerta: modo("Puerta de ruido"),
            modo_overdrive: modo("Overdrive"),
            modo_compresor: modo("Compresor"),
            modo_eq: modo("EQ"),
            modo_chorus: modo("Chorus"),
            modo_phaser: modo("Phaser"),
            modo_tremolo: modo("Trémolo"),
            modo_delay: modo("Delay"),
            modo_reverb: modo("Reverb"),
        }
    }
}

impl MusaParams {
    pub fn modo(&self, id: PedalId) -> &EnumParam<Modo> {
        match id {
            PedalId::Puerta => &self.modo_puerta,
            PedalId::Overdrive => &self.modo_overdrive,
            PedalId::Compresor => &self.modo_compresor,
            PedalId::Eq => &self.modo_eq,
            PedalId::Chorus => &self.modo_chorus,
            PedalId::Phaser => &self.modo_phaser,
            PedalId::Tremolo => &self.modo_tremolo,
            PedalId::Delay => &self.modo_delay,
            PedalId::Reverb => &self.modo_reverb,
        }
    }

    pub fn modos(&self) -> [Modo; PEDALES] {
        musa_catalogo::ORDEN_CADENA.map(|id| self.modo(id).value())
    }
}
