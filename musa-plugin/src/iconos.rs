use std::collections::HashMap;
use std::path::{Path, PathBuf};

use nih_plug_egui::egui::{ColorImage, Context, TextureHandle, TextureOptions};

pub fn rutas_por_id(dir: &Path) -> HashMap<String, PathBuf> {
    musa_catalogo::listar_presets(dir)
        .into_iter()
        .map(|(ruta, preset)| (preset.id, ruta.join("icono.png")))
        .collect()
}

pub fn cargar(ctx: &Context, id: &str, ruta: &Path) -> Option<TextureHandle> {
    let imagen = image::open(ruta).ok()?.to_rgba8();
    let tamano = [imagen.width() as usize, imagen.height() as usize];
    let color = ColorImage::from_rgba_unmultiplied(tamano, imagen.as_raw());
    Some(ctx.load_texture(format!("icono-{id}"), color, TextureOptions::LINEAR))
}
