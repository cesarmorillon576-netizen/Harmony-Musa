use std::num::NonZeroU32;
use std::sync::atomic::{AtomicI32, AtomicU16, AtomicU32, Ordering};
use std::sync::Arc;

use dsp::Cadena;
use presets::Aplicador;

use nih_plug::prelude::*;

pub mod dsp;
mod gui;
mod iconos;
pub mod params;
pub mod presets;

use params::MusaParams;

pub struct Estado {
    pub frecuencia: AtomicF32,
    pub vigente: AtomicI32,
    pub encendidos: AtomicU16,
    pub recargas: AtomicU32,
}

impl Default for Estado {
    fn default() -> Self {
        Self {
            frecuencia: AtomicF32::new(0.0),
            vigente: AtomicI32::new(-1),
            encendidos: AtomicU16::new(0),
            recargas: AtomicU32::new(0),
        }
    }
}

impl Estado {
    pub fn pedal_encendido(&self, posicion: usize) -> bool {
        self.encendidos.load(Ordering::Relaxed) & (1 << posicion) != 0
    }
}

pub fn recargar_presets(params: &MusaParams) {
    let en_disco: Vec<_> = musa_catalogo::listar_presets(&presets::dir_presets())
        .into_iter()
        .map(|(_, p)| p)
        .collect();
    if let Ok(mut guardados) = params.presets.write() {
        *guardados = presets::fusionar(&guardados, &en_disco);
    }
}

pub struct Musa {
    params: Arc<MusaParams>,
    estado: Arc<Estado>,
    cadena: Cadena,
    aplicador: Aplicador,
    recargas_vistas: u32,
    entrada_mono: bool,
}

impl Default for Musa {
    fn default() -> Self {
        Self {
            params: Arc::new(MusaParams::default()),
            estado: Arc::new(Estado::default()),
            cadena: Cadena::completa(),
            aplicador: Aplicador::default(),
            recargas_vistas: 0,
            entrada_mono: false,
        }
    }
}

impl Plugin for Musa {
    const NAME: &'static str = "Harmony Musa";
    const VENDOR: &'static str = "César";
    const URL: &'static str = "https://github.com/cesarmorillon576-netizen/Harmony-Musa";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(2),
            main_output_channels: NonZeroU32::new(2),
            ..AudioIOLayout::const_default()
        },
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(1),
            main_output_channels: NonZeroU32::new(2),
            ..AudioIOLayout::const_default()
        },
    ];

    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        gui::crear(self.params.clone(), self.estado.clone())
    }

    fn initialize(
        &mut self,
        audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.estado
            .frecuencia
            .store(buffer_config.sample_rate, Ordering::Relaxed);
        self.entrada_mono = audio_io_layout.main_input_channels == NonZeroU32::new(1);
        self.cadena.preparar(
            buffer_config.sample_rate,
            buffer_config.max_buffer_size as usize,
        );
        recargar_presets(&self.params);
        self.aplicador.invalidar();
        true
    }

    fn reset(&mut self) {
        self.cadena.reiniciar();
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let recargas = self.estado.recargas.load(Ordering::Acquire);
        if recargas != self.recargas_vistas {
            self.recargas_vistas = recargas;
            self.aplicador.invalidar();
        }
        if let Ok(presets) = self.params.presets.try_read() {
            let indice = self.params.preset.value().max(0) as usize;
            self.aplicador
                .actualizar(&mut self.cadena, &presets, indice, self.params.modos());
        }
        let vigente = self.aplicador.vigente().map_or(-1, |v| v as i32);
        self.estado.vigente.store(vigente, Ordering::Relaxed);
        let bits = self
            .aplicador
            .efectivo()
            .iter()
            .enumerate()
            .fold(0u16, |b, (i, &e)| b | (u16::from(e) << i));
        self.estado.encendidos.store(bits, Ordering::Relaxed);

        let canales = buffer.as_slice();
        let (izq, resto) = canales.split_at_mut(1);
        let (izq, der) = (&mut *izq[0], &mut *resto[0]);
        if self.entrada_mono {
            der.copy_from_slice(izq);
        }
        self.cadena.procesar(izq, der);
        ProcessStatus::Normal
    }
}

impl ClapPlugin for Musa {
    const CLAP_ID: &'static str = "com.cesar.harmony-musa";
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("Pedalera multiefectos para guitarra con presets diseñados por IA");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Stereo,
        ClapFeature::Mono,
        ClapFeature::MultiEffects,
    ];
}

impl Vst3Plugin for Musa {
    const VST3_CLASS_ID: [u8; 16] = *b"HarmonyMusaPedal";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Modulation];
}

nih_export_clap!(Musa);
nih_export_vst3!(Musa);
