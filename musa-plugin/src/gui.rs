use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use musa_catalogo::{Preset, ORDEN_CADENA, PEDALES};
use nih_plug::prelude::*;
use nih_plug_egui::create_egui_editor;
use nih_plug_egui::egui::{
    self, epaint::Mesh, Align2, Color32, Context, CornerRadius, FontId, Pos2, Rect, Sense, Stroke,
    StrokeKind, TextureHandle, Ui, Vec2,
};

use crate::iconos;
use crate::params::{Modo, MusaParams};
use crate::presets::dir_presets;
use crate::{recargar_presets, Estado};

pub const ANCHO: u32 = 760;
pub const ALTO: u32 = 560;

const BARNIZ_ARRIBA: Color32 = Color32::from_rgb(0x24, 0x1a, 0x15);
const BARNIZ_ABAJO: Color32 = Color32::from_rgb(0x11, 0x0c, 0x0a);
const TERRACOTA: Color32 = Color32::from_rgb(0xc7, 0x76, 0x4a);
const OCRE: Color32 = Color32::from_rgb(0xe2, 0xb4, 0x7c);
const MARFIL: Color32 = Color32::from_rgb(0xf1, 0xe3, 0xc8);
const ORO: Color32 = Color32::from_rgb(0xe6, 0xc2, 0x7a);
const TEXTO_TENUE: Color32 = Color32::from_rgb(0x9c, 0x85, 0x74);
const APAGADO: Color32 = Color32::from_rgb(0x4a, 0x3b, 0x33);

const NOMBRES_PEDALES: [&str; PEDALES] = [
    "Puerta",
    "Overdrive",
    "Compresor",
    "EQ",
    "Chorus",
    "Phaser",
    "Trémolo",
    "Delay",
    "Reverb",
];

#[derive(Default)]
struct EstadoGui {
    rutas: HashMap<String, PathBuf>,
    iconos: HashMap<String, Option<TextureHandle>>,
}

impl EstadoGui {
    fn recargar(&mut self, params: &MusaParams, estado: &Estado) {
        recargar_presets(params);
        estado.recargas.fetch_add(1, Ordering::Release);
        self.rutas = iconos::rutas_por_id(&dir_presets());
        self.iconos.clear();
    }

    fn icono(&mut self, ctx: &Context, id: &str) -> Option<TextureHandle> {
        if !self.iconos.contains_key(id) {
            let textura = self
                .rutas
                .get(id)
                .and_then(|ruta| iconos::cargar(ctx, id, ruta));
            self.iconos.insert(id.to_string(), textura);
        }
        self.iconos.get(id).cloned().flatten()
    }
}

fn con_alfa(color: Color32, alfa: u8) -> Color32 {
    color.gamma_multiply(alfa as f32 / 255.0)
}

fn espaciado(texto: &str) -> String {
    texto
        .chars()
        .map(|c| c.to_string())
        .collect::<Vec<_>>()
        .join("\u{2009}\u{2009}")
}

fn pintar_fondo(ui: &Ui) {
    let rect = ui.ctx().screen_rect();
    let mut malla = Mesh::default();
    for (pos, color) in [
        (rect.left_top(), BARNIZ_ARRIBA),
        (rect.right_top(), BARNIZ_ARRIBA),
        (rect.right_bottom(), BARNIZ_ABAJO),
        (rect.left_bottom(), BARNIZ_ABAJO),
    ] {
        malla.colored_vertex(pos, color);
    }
    malla.add_triangle(0, 1, 2);
    malla.add_triangle(0, 2, 3);
    ui.painter().add(malla);
}

fn disco_radial(ui: &Ui, centro: Pos2, radio: f32, dentro: Color32, fuera: Color32) {
    let mut malla = Mesh::default();
    malla.colored_vertex(centro, dentro);
    let lados = 64;
    for i in 0..=lados {
        let angulo = i as f32 / lados as f32 * std::f32::consts::TAU;
        malla.colored_vertex(centro + Vec2::angled(angulo) * radio, fuera);
    }
    for i in 1..=lados as u32 {
        malla.add_triangle(0, i, i + 1);
    }
    ui.painter().add(malla);
}

fn greca(ui: &Ui, izquierda: f32, derecha: f32, y: f32) {
    let alto = 10.0;
    let paso = 20.0;
    let trazo = Stroke::new(1.4, con_alfa(TERRACOTA, 200));
    let pintor = ui.painter();
    pintor.line_segment(
        [Pos2::new(izquierda, y - 3.0), Pos2::new(derecha, y - 3.0)],
        Stroke::new(1.0, con_alfa(TERRACOTA, 120)),
    );
    pintor.line_segment(
        [
            Pos2::new(izquierda, y + alto + 3.0),
            Pos2::new(derecha, y + alto + 3.0),
        ],
        Stroke::new(1.0, con_alfa(TERRACOTA, 120)),
    );
    let mut x = izquierda;
    while x + paso <= derecha {
        let puntos = [
            Pos2::new(x, y + alto),
            Pos2::new(x, y),
            Pos2::new(x + paso * 0.7, y),
            Pos2::new(x + paso * 0.7, y + alto * 0.65),
            Pos2::new(x + paso * 0.35, y + alto * 0.65),
            Pos2::new(x + paso * 0.35, y + alto * 0.3),
        ];
        pintor.add(egui::Shape::line(puntos.to_vec(), trazo));
        pintor.line_segment(
            [Pos2::new(x, y + alto), Pos2::new(x + paso, y + alto)],
            trazo,
        );
        x += paso;
    }
}

fn lira(ui: &Ui, centro: Pos2, alto: f32, color: Color32) {
    use egui::epaint::QuadraticBezierShape;
    let a = alto / 2.0;
    let grosor = (alto / 22.0).max(1.0);
    let trazo = Stroke::new(grosor, color);
    let pintor = ui.painter();
    for lado in [-1.0f32, 1.0] {
        let brazo = [
            centro + Vec2::new(lado * a * 0.28, a * 0.62),
            centro + Vec2::new(lado * a * 0.95, a * 0.35),
            centro + Vec2::new(lado * a * 0.42, -a * 0.72),
        ];
        pintor.add(QuadraticBezierShape::from_points_stroke(
            brazo,
            false,
            Color32::TRANSPARENT,
            trazo,
        ));
        let rizo = [
            centro + Vec2::new(lado * a * 0.42, -a * 0.72),
            centro + Vec2::new(lado * a * 0.30, -a * 1.0),
            centro + Vec2::new(lado * a * 0.62, -a * 0.95),
        ];
        pintor.add(QuadraticBezierShape::from_points_stroke(
            rizo,
            false,
            Color32::TRANSPARENT,
            trazo,
        ));
    }
    let base = [
        centro + Vec2::new(-a * 0.28, a * 0.62),
        centro + Vec2::new(0.0, a * 0.95),
        centro + Vec2::new(a * 0.28, a * 0.62),
    ];
    pintor.add(QuadraticBezierShape::from_points_stroke(
        base,
        false,
        Color32::TRANSPARENT,
        trazo,
    ));
    let barra_y = -a * 0.58;
    pintor.line_segment(
        [
            centro + Vec2::new(-a * 0.52, barra_y),
            centro + Vec2::new(a * 0.52, barra_y),
        ],
        Stroke::new(grosor * 1.3, color),
    );
    let cuerdas = 5;
    for i in 0..cuerdas {
        let x = (i as f32 / (cuerdas - 1) as f32 - 0.5) * a * 0.44;
        pintor.line_segment(
            [
                centro + Vec2::new(x, barra_y),
                centro + Vec2::new(x * 0.8, a * 0.66),
            ],
            Stroke::new(grosor * 0.55, color.gamma_multiply(0.8)),
        );
    }
}

fn boton(ui: &mut Ui, texto: &str) -> bool {
    let galeria =
        ui.painter()
            .layout_no_wrap(texto.to_string(), FontId::proportional(13.0), MARFIL);
    let tamano = galeria.size() + Vec2::new(26.0, 12.0);
    let (rect, respuesta) = ui.allocate_exact_size(tamano, Sense::click());
    let fondo = if respuesta.hovered() {
        con_alfa(TERRACOTA, 60)
    } else {
        con_alfa(TERRACOTA, 25)
    };
    ui.painter()
        .rect_filled(rect, CornerRadius::same(14), fondo);
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(14),
        Stroke::new(1.0, con_alfa(TERRACOTA, 160)),
        StrokeKind::Inside,
    );
    ui.painter()
        .galley(rect.center() - galeria.size() / 2.0, galeria, MARFIL);
    respuesta.clicked()
}

fn imagen_circular(ui: &Ui, textura: &TextureHandle, centro: Pos2, radio: f32) {
    let rect = Rect::from_center_size(centro, Vec2::splat(radio * 2.0));
    egui::Image::new((textura.id(), rect.size()))
        .corner_radius(CornerRadius::same(radio.min(255.0) as u8))
        .paint_at(ui, rect);
}

fn esfera_vacia(ui: &Ui, centro: Pos2, radio: f32, preset: Option<&Preset>) {
    disco_radial(
        ui,
        centro,
        radio,
        Color32::from_rgb(0x8a, 0x5a, 0x3c),
        Color32::from_rgb(0x2b, 0x1c, 0x15),
    );
    disco_radial(
        ui,
        centro - Vec2::splat(radio * 0.3),
        radio * 0.45,
        con_alfa(MARFIL, 60),
        Color32::TRANSPARENT,
    );
    let Some(inicial) = preset
        .and_then(|p| p.nombre.chars().next())
        .map(|c| c.to_uppercase().to_string())
    else {
        lira(ui, centro, radio * 1.1, con_alfa(MARFIL, 190));
        return;
    };
    ui.painter().text(
        centro,
        Align2::CENTER_CENTER,
        inicial,
        FontId::proportional(radio * 0.7),
        con_alfa(MARFIL, 200),
    );
}

fn medallon_central(
    ui: &mut Ui,
    centro: Pos2,
    preset: Option<&Preset>,
    icono: Option<TextureHandle>,
) {
    let radio = 92.0;
    disco_radial(
        ui,
        centro,
        radio * 1.9,
        con_alfa(ORO, 26),
        Color32::TRANSPARENT,
    );
    let pintor = ui.painter();
    let rayos = 28;
    for i in 0..rayos {
        let angulo = i as f32 / rayos as f32 * std::f32::consts::TAU;
        let largo = if i % 2 == 0 { 22.0 } else { 13.0 };
        let base = radio + 14.0;
        let punta = centro + Vec2::angled(angulo) * (base + largo);
        let lado = Vec2::angled(angulo + std::f32::consts::FRAC_PI_2) * 3.5;
        let raiz = centro + Vec2::angled(angulo) * base;
        pintor.add(egui::Shape::convex_polygon(
            vec![raiz + lado, punta, raiz - lado],
            con_alfa(TERRACOTA, 190),
            Stroke::NONE,
        ));
    }
    pintor.circle_filled(centro, radio + 11.0, BARNIZ_ABAJO);
    pintor.circle_stroke(
        centro,
        radio + 11.0,
        Stroke::new(1.2, con_alfa(TERRACOTA, 220)),
    );
    let cuentas = 36;
    for i in 0..cuentas {
        let angulo = i as f32 / cuentas as f32 * std::f32::consts::TAU;
        pintor.circle_filled(
            centro + Vec2::angled(angulo) * (radio + 5.5),
            1.8,
            con_alfa(OCRE, 210),
        );
    }
    match &icono {
        Some(textura) => imagen_circular(ui, textura, centro, radio),
        None => esfera_vacia(ui, centro, radio, preset),
    }
    ui.painter()
        .circle_stroke(centro, radio, Stroke::new(1.0, con_alfa(ORO, 150)));
}

fn medallon(ui: &mut Ui, preset: &Preset, activo: bool, icono: Option<TextureHandle>) -> bool {
    let (rect, respuesta) = ui.allocate_exact_size(Vec2::new(70.0, 82.0), Sense::click());
    let radio = 26.0;
    let centro = Pos2::new(rect.center().x, rect.top() + radio + 4.0);
    if activo {
        disco_radial(
            ui,
            centro,
            radio * 1.9,
            con_alfa(ORO, 70),
            Color32::TRANSPARENT,
        );
    }
    ui.painter()
        .circle_filled(centro, radio + 3.0, BARNIZ_ABAJO);
    match &icono {
        Some(textura) => imagen_circular(ui, textura, centro, radio),
        None => esfera_vacia(ui, centro, radio, Some(preset)),
    }
    let borde = match (activo, respuesta.hovered()) {
        (true, _) => Stroke::new(2.0, ORO),
        (false, true) => Stroke::new(1.5, con_alfa(OCRE, 200)),
        (false, false) => Stroke::new(1.0, con_alfa(TERRACOTA, 150)),
    };
    ui.painter().circle_stroke(centro, radio + 3.0, borde);
    let color = if activo { MARFIL } else { TEXTO_TENUE };
    let nombre = ui.painter().layout(
        preset.nombre.clone(),
        FontId::proportional(11.0),
        color,
        rect.width(),
    );
    let nombre = if nombre.rows.len() > 1 {
        let corto: String = preset.nombre.chars().take(10).collect();
        ui.painter()
            .layout_no_wrap(format!("{corto}…"), FontId::proportional(11.0), color)
    } else {
        nombre
    };
    let pos = Pos2::new(
        rect.center().x - nombre.size().x / 2.0,
        rect.bottom() - 14.0,
    );
    ui.painter().galley(pos, nombre, color);
    respuesta.on_hover_text(&preset.nombre).clicked()
}

fn carrusel(
    ui: &mut Ui,
    presets: &[Preset],
    vigente: Option<usize>,
    gui: &mut EstadoGui,
    params: &MusaParams,
    setter: &ParamSetter,
) {
    if presets.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(24.0);
            ui.label(
                egui::RichText::new("Genera un preset con la app y pulsa Recargar")
                    .color(TEXTO_TENUE),
            );
        });
        return;
    }
    egui::ScrollArea::horizontal()
        .id_salt("carrusel")
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                let ancho_total = presets.len() as f32 * 76.0;
                let margen = ((ui.available_width() - ancho_total) / 2.0).max(0.0);
                ui.add_space(margen);
                for (i, preset) in presets.iter().enumerate() {
                    let icono = gui.icono(ui.ctx(), &preset.id);
                    if medallon(ui, preset, vigente == Some(i), icono) {
                        setter.begin_set_parameter(&params.preset);
                        setter.set_parameter(&params.preset, i as i32);
                        setter.end_set_parameter(&params.preset);
                    }
                }
            });
        });
}

fn chip(ui: &mut Ui, texto: &str, elegido: bool) -> bool {
    let (rect, respuesta) = ui.allocate_exact_size(Vec2::new(22.0, 15.0), Sense::click());
    let fondo = if elegido {
        con_alfa(TERRACOTA, 150)
    } else if respuesta.hovered() {
        con_alfa(TERRACOTA, 40)
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, CornerRadius::same(4), fondo);
    let color = if elegido { MARFIL } else { TEXTO_TENUE };
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        texto,
        FontId::proportional(9.0),
        color,
    );
    respuesta.clicked()
}

fn pedalera(ui: &mut Ui, params: &MusaParams, estado: &Estado, setter: &ParamSetter) {
    let separacion = 6.0;
    let ancho = (ui.available_width() - (PEDALES - 1) as f32 * separacion) / PEDALES as f32;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = separacion;
        for (k, id) in ORDEN_CADENA.iter().enumerate() {
            let (rect, _) = ui.allocate_exact_size(Vec2::new(ancho, 54.0), Sense::hover());
            let encendido = estado.pedal_encendido(k);
            let pintor = ui.painter();
            pintor.rect_filled(
                rect,
                CornerRadius::same(6),
                con_alfa(TERRACOTA, if encendido { 34 } else { 12 }),
            );
            pintor.rect_stroke(
                rect,
                CornerRadius::same(6),
                Stroke::new(1.0, con_alfa(TERRACOTA, if encendido { 170 } else { 70 })),
                StrokeKind::Inside,
            );
            let led = Pos2::new(rect.left() + 10.0, rect.top() + 16.0);
            if encendido {
                disco_radial(ui, led, 9.0, con_alfa(OCRE, 120), Color32::TRANSPARENT);
                ui.painter().circle_filled(led, 3.5, MARFIL);
            } else {
                ui.painter().circle_filled(led, 3.0, APAGADO);
            }
            let color = if encendido { MARFIL } else { TEXTO_TENUE };
            ui.painter().text(
                Pos2::new(led.x + 8.0, led.y),
                Align2::LEFT_CENTER,
                NOMBRES_PEDALES[k],
                FontId::proportional(11.5),
                color,
            );

            let param = params.modo(*id);
            let actual = param.value();
            let fila = Rect::from_center_size(
                Pos2::new(rect.center().x, rect.bottom() - 13.0),
                Vec2::new(70.0, 15.0),
            );
            let mut hijo = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(fila)
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
            );
            hijo.spacing_mut().item_spacing.x = 2.0;
            for (texto, modo) in [
                ("auto", Modo::SegunPreset),
                ("on", Modo::Encendido),
                ("off", Modo::Apagado),
            ] {
                if chip(&mut hijo, texto, actual == modo) && actual != modo {
                    setter.begin_set_parameter(param);
                    setter.set_parameter(param, modo);
                    setter.end_set_parameter(param);
                }
            }
        }
    });
}

fn vista(
    ui: &mut Ui,
    presets: &[Preset],
    vigente: Option<usize>,
    gui: &mut EstadoGui,
    params: &MusaParams,
    estado: &Estado,
    setter: &ParamSetter,
) {
    pintar_fondo(ui);
    let marco = ui.max_rect();
    let activo = vigente.and_then(|i| presets.get(i));

    let titulo_y = marco.top() + 14.0;
    ui.painter().text(
        Pos2::new(marco.center().x, titulo_y),
        Align2::CENTER_CENTER,
        espaciado("HARMONY MUSA"),
        FontId::proportional(15.0),
        MARFIL,
    );
    for lado in [-1.0, 1.0] {
        let x = marco.center().x + lado * 112.0;
        lira(ui, Pos2::new(x, titulo_y), 30.0, TERRACOTA);
    }
    let mut cabecera = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(Rect::from_min_size(
                marco.right_top() - Vec2::new(110.0, 0.0),
                Vec2::new(110.0, 30.0),
            ))
            .layout(egui::Layout::right_to_left(egui::Align::TOP)),
    );
    if boton(&mut cabecera, "Recargar") {
        gui.recargar(params, estado);
    }
    greca(ui, marco.left(), marco.right(), marco.top() + 40.0);

    let centro = Pos2::new(marco.center().x, marco.top() + 178.0);
    let icono = activo.and_then(|p| gui.icono(ui.ctx(), &p.id));
    medallon_central(ui, centro, activo, icono);

    let (nombre, descripcion) = activo
        .map(|p| (p.nombre.as_str(), p.descripcion.as_str()))
        .unwrap_or((
            "Sin preset",
            "Elige un preset o genera uno nuevo con la app",
        ));
    let y_nombre = centro.y + 136.0;
    ui.painter().text(
        Pos2::new(marco.center().x, y_nombre),
        Align2::CENTER_CENTER,
        nombre,
        FontId::proportional(20.0),
        MARFIL,
    );
    ui.painter().text(
        Pos2::new(marco.center().x, y_nombre + 22.0),
        Align2::CENTER_CENTER,
        descripcion,
        FontId::proportional(12.0),
        TEXTO_TENUE,
    );

    let zona_carrusel = Rect::from_min_max(
        Pos2::new(marco.left(), y_nombre + 40.0),
        Pos2::new(marco.right(), y_nombre + 126.0),
    );
    let mut hijo = ui.new_child(egui::UiBuilder::new().max_rect(zona_carrusel));
    carrusel(&mut hijo, presets, vigente, gui, params, setter);

    let y_cadena = marco.bottom() - 58.0;
    ui.painter().line_segment(
        [
            Pos2::new(marco.left(), y_cadena - 10.0),
            Pos2::new(marco.right(), y_cadena - 10.0),
        ],
        Stroke::new(1.0, con_alfa(TERRACOTA, 90)),
    );
    let mut hijo = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_max(
        Pos2::new(marco.left(), y_cadena),
        marco.right_bottom(),
    )));
    pedalera(&mut hijo, params, estado, setter);
}

pub fn crear(params: Arc<MusaParams>, estado: Arc<Estado>) -> Option<Box<dyn Editor>> {
    let params_build = params.clone();
    let estado_build = estado.clone();
    create_egui_editor(
        params.editor_state.clone(),
        EstadoGui::default(),
        move |ctx, gui| {
            let mut visuales = egui::Visuals::dark();
            visuales.panel_fill = Color32::TRANSPARENT;
            visuales.extreme_bg_color = BARNIZ_ABAJO;
            visuales.override_text_color = Some(MARFIL);
            ctx.set_visuals(visuales);
            gui.recargar(&params_build, &estado_build);
        },
        move |ctx, setter, gui| {
            let presets: Vec<Preset> = params.presets.read().map(|p| p.clone()).unwrap_or_default();
            let vigente = usize::try_from(estado.vigente.load(Ordering::Relaxed)).ok();
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE.inner_margin(egui::Margin::same(18)))
                .show(ctx, |ui| {
                    vista(ui, &presets, vigente, gui, &params, &estado, setter);
                });
            ctx.request_repaint_after(std::time::Duration::from_millis(120));
        },
    )
}
