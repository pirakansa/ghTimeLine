use eframe::egui;

const NOTO_SANS_JP_FONT_ID: &str = "noto_sans_jp";

pub fn install_fonts(ctx: &egui::Context) {
    ctx.set_fonts(noto_sans_font_definitions());
}

fn noto_sans_font_definitions() -> egui::FontDefinitions {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        NOTO_SANS_JP_FONT_ID.to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../../assets/NotoSansCJKjp-Regular.otf"
        ))),
    );

    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, NOTO_SANS_JP_FONT_ID.to_owned());
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .insert(0, NOTO_SANS_JP_FONT_ID.to_owned());

    fonts
}
