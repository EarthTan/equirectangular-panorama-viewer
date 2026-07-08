use crate::error::LoadError;
use egui::{
    Align2, Color32, Context, FontData, FontDefinitions, FontId, LayerId, Pos2, RichText, Stroke,
    StrokeKind, Vec2,
};
use std::path::{Path, PathBuf};
use std::time::Instant;

#[derive(Debug, Default)]
pub struct UiState {
    pub error_text: Option<String>,
    pub error_show_until: Option<std::time::Instant>,
    pub hud_show_until: Option<std::time::Instant>,
    pub has_panorama: bool,
    pub loading: Option<PathBuf>,
    pub loading_started_at: Option<Instant>,
}

impl UiState {
    pub fn show_error(&mut self, message: String, duration_ms: u64) {
        self.error_text = Some(message);
        self.error_show_until =
            Some(std::time::Instant::now() + std::time::Duration::from_millis(duration_ms));
    }

    pub fn show_hud(&mut self, duration_ms: u64) {
        self.hud_show_until =
            Some(std::time::Instant::now() + std::time::Duration::from_millis(duration_ms));
    }

    pub fn show_panorama_loaded(&mut self) {
        self.has_panorama = true;
        self.show_hud(3000);
    }

    #[allow(dead_code)]
    pub fn show_panorama_replaced(&mut self) {
        self.show_hud(3000);
    }

    pub fn begin_loading(&mut self, path: PathBuf) {
        self.loading = Some(path);
        self.loading_started_at = Some(Instant::now());
    }

    pub fn clear_loading(&mut self) {
        self.loading = None;
        self.loading_started_at = None;
    }

    pub fn error_for_load_error(e: &LoadError) -> String {
        match e {
            LoadError::NotAnImage(_) => "请拖入图片文件".to_string(),
            LoadError::Decode { .. } => "图片加载失败".to_string(),
            LoadError::Io(_, _) => "图片加载失败".to_string(),
        }
    }
}

/// Configure egui with a CJK-capable font loaded from the system.
/// Tries several common locations per platform; falls back to egui's default
/// if none found.
pub fn install_fonts(ctx: &Context) {
    let mut fonts = FontDefinitions::default();
    let candidates: &[&str] = &[
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/opentype/noto/NotoSansCJK.ttc",
        "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttf",
        "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
        "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",
        "/usr/share/fonts/truetype/arphic/uming.ttc",
        "/usr/share/fonts/truetype/arphic/ukai.ttc",
        "/Library/Fonts/Arial Unicode.ttf",
        "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
        "/System/Library/Fonts/Hiragino Sans GB.ttc",
        "/System/Library/Fonts/STHeiti Medium.ttc",
        "/System/Library/Fonts/STHeiti Light.ttc",
        "C:/Windows/Fonts/msyh.ttc",
        "C:/Windows/Fonts/msyh.ttf",
        "C:/Windows/Fonts/simsun.ttc",
        "C:/Windows/Fonts/simhei.ttf",
    ];
    for path in candidates {
        if let Ok(bytes) = std::fs::read(path) {
            fonts.font_data.insert(
                "cjk".to_owned(),
                std::sync::Arc::new(FontData::from_owned(bytes)),
            );
            fonts
                .families
                .get_mut(&egui::FontFamily::Proportional)
                .unwrap()
                .insert(0, "cjk".to_owned());
            fonts
                .families
                .get_mut(&egui::FontFamily::Monospace)
                .unwrap()
                .insert(0, "cjk".to_owned());
            log::info!("loaded CJK font from {path}");
            break;
        } else {
            log::debug!("CJK font not found at {path}");
        }
    }
    ctx.set_fonts(fonts);
}

/// What the UI wants the application to do as a result of this frame.
#[derive(Debug, Default)]
pub struct UiOutput {
    pub open_file_picker: bool,
}

pub fn draw(ctx: &Context, state: &UiState) -> UiOutput {
    let mut out = UiOutput::default();
    let now = std::time::Instant::now();
    let screen = ctx.content_rect();

    if state.loading.is_some() {
        ctx.request_repaint_after(std::time::Duration::from_millis(33));
    }

    // Error banner.
    if let (Some(text), Some(until)) = (&state.error_text, state.error_show_until) {
        if now < until {
            let painter = ctx.layer_painter(LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("error_banner_layer"),
            ));
            let font_id = FontId::proportional(14.0);
            let galley = painter.layout_no_wrap(text.clone(), font_id, Color32::WHITE);
            let padding = Vec2::new(20.0, 10.0);
            let size = galley.size() + padding * 2.0;
            let pos = Pos2::new((screen.width() - size.x) / 2.0, 16.0);
            let rect = egui::Rect::from_min_size(pos, size);
            painter.rect_filled(rect, 8.0, Color32::from_rgb(229, 72, 77));
            painter.galley(rect.min + padding, galley, Color32::WHITE);
        }
    }

    // Empty state (drag-drop prompt). Hidden while loading.
    if !state.has_panorama && state.loading.is_none() {
        let painter = ctx.layer_painter(LayerId::new(
            egui::Order::Background,
            egui::Id::new("empty_state_layer"),
        ));
        painter.rect_filled(screen, 0.0, Color32::from_rgb(10, 10, 10));

        let frame =
            egui::Rect::from_center_size(screen.center(), Vec2::new(screen.width() * 0.4, 240.0));
        painter.rect_stroke(
            frame.expand(2.0),
            0.0,
            Stroke::new(2.0, Color32::from_rgb(42, 42, 42)),
            StrokeKind::Inside,
        );

        let title = RichText::new("拖入一张全景图")
            .font(FontId::proportional(20.0))
            .color(Color32::from_rgb(224, 224, 224));
        let sub = RichText::new("或将图片拖到此处 · 点击选择文件")
            .font(FontId::proportional(14.0))
            .color(Color32::from_rgb(136, 136, 136));

        painter.text(
            frame.center() + Vec2::new(0.0, -10.0),
            Align2::CENTER_CENTER,
            title.text(),
            FontId::proportional(20.0),
            Color32::from_rgb(224, 224, 224),
        );
        painter.text(
            frame.center() + Vec2::new(0.0, 16.0),
            Align2::CENTER_CENTER,
            sub.text(),
            FontId::proportional(14.0),
            Color32::from_rgb(136, 136, 136),
        );

        let click_area = egui::Area::new(egui::Id::new("empty_state_click_area"))
            .fixed_pos(frame.shrink(8.0).min)
            .constrain(false)
            .interactable(true);
        click_area.show(ctx, |ui| {
            let rect = egui::Rect::from_min_size(Pos2::ZERO, frame.shrink(8.0).size());
            let (_, response) = ui.allocate_exact_size(rect.size(), egui::Sense::click());
            if response.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if response.clicked() {
                out.open_file_picker = true;
            }
        });
    }

    // Loading overlay.
    if let Some(path) = &state.loading {
        let started = state.loading_started_at.unwrap_or(now);
        draw_loading_overlay(ctx, path, started, now);
    }

    // "Open file" button.
    if state.has_panorama {
        let label = "打开文件";
        let font_id = FontId::proportional(13.0);
        let painter = ctx.layer_painter(LayerId::new(
            egui::Order::Foreground,
            egui::Id::new("open_file_button_layer"),
        ));
        let galley =
            painter.layout_no_wrap(label.to_string(), font_id, Color32::from_rgb(224, 224, 224));
        let padding = Vec2::new(14.0, 8.0);
        let size = galley.size() + padding * 2.0;
        let pos = Pos2::new(16.0, 16.0);
        let button_rect = egui::Rect::from_min_size(pos, size);

        painter.rect_filled(
            button_rect,
            8.0,
            Color32::from_rgba_unmultiplied(10, 10, 10, 178),
        );
        painter.rect_stroke(
            button_rect,
            8.0,
            Stroke::new(1.0, Color32::from_rgb(58, 58, 58)),
            StrokeKind::Inside,
        );
        painter.galley(
            button_rect.min + padding,
            galley,
            Color32::from_rgb(224, 224, 224),
        );

        let area = egui::Area::new(egui::Id::new("open_file_button_area"))
            .fixed_pos(button_rect.min)
            .constrain(false)
            .interactable(true);
        area.show(ctx, |ui| {
            let (_, response) = ui.allocate_exact_size(button_rect.size(), egui::Sense::click());
            if response.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if response.clicked() {
                out.open_file_picker = true;
            }
        });
    }

    // HUD.
    if let Some(until) = state.hud_show_until {
        if now < until {
            let painter = ctx.layer_painter(LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("hud_layer"),
            ));
            let text = "拖动旋转 · 滚轮缩放";
            let font_id = FontId::proportional(13.0);
            let galley =
                painter.layout_no_wrap(text.to_string(), font_id, Color32::from_rgb(136, 136, 136));
            let padding = Vec2::new(16.0, 8.0);
            let size = galley.size() + padding * 2.0;
            let pos = Pos2::new(
                (screen.width() - size.x) / 2.0,
                screen.height() - size.y - 24.0,
            );
            let rect = egui::Rect::from_min_size(pos, size);
            painter.rect_filled(rect, 20.0, Color32::from_rgba_unmultiplied(10, 10, 10, 153));
            painter.galley(rect.min + padding, galley, Color32::from_rgb(136, 136, 136));
        }
    }
    out
}

/// Draw a centered card with a rotating arc spinner plus the file name being
/// loaded. Pure paint — no interaction; the user can still swap files via
/// the "打开文件" button or drag-drop, which will cancel this load.
fn draw_loading_overlay(ctx: &Context, path: &Path, started: Instant, now: Instant) {
    let screen = ctx.content_rect();

    // Backdrop dim, so the underlying scene (or empty background) reads as
    // "behind glass" and the spinner pops.
    let painter = ctx.layer_painter(LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("loading_backdrop_layer"),
    ));
    painter.rect_filled(screen, 0.0, Color32::from_rgba_unmultiplied(0, 0, 0, 110));

    // Card.
    let card_w = 280.0_f32.min(screen.width() - 48.0);
    let card_h = 132.0_f32;
    let card = egui::Rect::from_center_size(screen.center(), Vec2::new(card_w, card_h));
    painter.rect_filled(card, 12.0, Color32::from_rgba_unmultiplied(20, 20, 20, 230));
    painter.rect_stroke(
        card,
        12.0,
        Stroke::new(1.0, Color32::from_rgb(58, 58, 58)),
        StrokeKind::Inside,
    );

    // Rotating spinner. 270° arc that revolves once per second.
    let spinner_center = card.min + Vec2::new(34.0, card_h * 0.5);
    let spinner_radius = 14.0_f32;
    let elapsed = now.duration_since(started).as_secs_f32();
    let start_angle = elapsed * std::f32::consts::TAU;
    let sweep = 270.0_f32.to_radians();
    let arc_color = Color32::from_rgb(224, 224, 224);
    let track_color = Color32::from_rgba_unmultiplied(255, 255, 255, 28);
    // Track (full faint ring).
    painter.circle_stroke(
        spinner_center,
        spinner_radius,
        Stroke::new(2.5, track_color),
    );
    // Spinning arc.
    let n_segments = 48;
    let mut prev = point_on_circle(spinner_center, spinner_radius, start_angle);
    for i in 1..=n_segments {
        let t = i as f32 / n_segments as f32;
        let angle = start_angle + sweep * t;
        let p = point_on_circle(spinner_center, spinner_radius, angle);
        // Fade alpha along the arc length so it tapers.
        let alpha = (255.0 * (1.0 - t * 0.7)) as u8;
        painter.line_segment(
            [prev, p],
            Stroke::new(
                2.5,
                Color32::from_rgba_unmultiplied(arc_color.r(), arc_color.g(), arc_color.b(), alpha),
            ),
        );
        prev = p;
    }

    // Title + file name + ellipsis dots that animate.
    let title_pos = card.min + Vec2::new(64.0, 34.0);
    painter.text(
        title_pos,
        Align2::LEFT_TOP,
        "正在加载",
        FontId::proportional(14.0),
        Color32::from_rgb(224, 224, 224),
    );

    // File name (basename only — full path can be very long).
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("(未知文件)");
    let sub_pos = card.min + Vec2::new(64.0, 60.0);
    let max_w = card.width() - 80.0;
    let name_font = FontId::proportional(12.0);
    let name_galley = painter.layout(
        name.to_string(),
        name_font.clone(),
        Color32::from_rgb(160, 160, 160),
        max_w,
    );
    painter.galley(sub_pos, name_galley, Color32::from_rgb(160, 160, 160));

    // Animated ellipsis to reinforce that work is in progress.
    let dots_n = ((elapsed * 2.0).floor() as usize % 4).max(1);
    let dots: String = std::iter::repeat_n('.', dots_n).collect();
    let dots_pos = card.min + Vec2::new(card.width() - 28.0, 34.0);
    painter.text(
        dots_pos,
        Align2::CENTER_TOP,
        &dots,
        FontId::proportional(14.0),
        Color32::from_rgb(160, 160, 160),
    );
}

fn point_on_circle(center: Pos2, radius: f32, angle_rad: f32) -> Pos2 {
    Pos2::new(
        center.x + radius * angle_rad.cos(),
        center.y + radius * angle_rad.sin(),
    )
}
