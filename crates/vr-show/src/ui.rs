use crate::error::LoadError;
use egui::{
    Align2, Color32, Context, FontData, FontDefinitions, FontId, LayerId, Pos2, RichText, Stroke,
    StrokeKind, Vec2,
};

#[derive(Debug)]
pub struct UiState {
    pub error_text: Option<String>,
    pub error_show_until: Option<std::time::Instant>,
    pub hud_show_until: Option<std::time::Instant>,
    pub has_panorama: bool,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            error_text: None,
            error_show_until: None,
            hud_show_until: None,
            has_panorama: false,
        }
    }
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

    pub fn show_panorama_replaced(&mut self) {
        self.show_hud(3000);
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
/// Tries several common locations; falls back to egui's default if none found.
pub fn install_fonts(ctx: &Context) {
    let mut fonts = FontDefinitions::default();
    // Try several candidate system CJK font files (in order of preference).
    let candidates: &[&str] = &[
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/opentype/noto/NotoSansCJK.ttc",
        "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
        "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",
        "/usr/share/fonts/truetype/arphic/uming.ttc",
        "/usr/share/fonts/truetype/arphic/ukai.ttc",
    ];
    for path in candidates {
        if let Ok(bytes) = std::fs::read(path) {
            fonts.font_data.insert(
                "cjk".to_owned(),
                std::sync::Arc::new(FontData::from_owned(bytes)),
            );
            // Insert into the proportional family so it is tried for any text.
            fonts
                .families
                .get_mut(&egui::FontFamily::Proportional)
                .unwrap()
                .insert(0, "cjk".to_owned());
            // And for monospace.
            fonts
                .families
                .get_mut(&egui::FontFamily::Monospace)
                .unwrap()
                .insert(0, "cjk".to_owned());
            log::info!("loaded CJK font from {path}");
            break;
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

    // Empty state.
    if !state.has_panorama {
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

        // Invisible click target covering the whole card so the user can
        // click anywhere on the empty state to open a file picker.
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
