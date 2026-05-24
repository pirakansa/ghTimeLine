use std::collections::HashMap;
use std::sync::mpsc;

use eframe::egui;

const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));
const FETCH_SIZE_PX: u32 = 64;

#[derive(Default)]
pub struct AvatarCache {
    entries: HashMap<String, CacheEntry>,
    tx: Option<mpsc::Sender<AvatarResult>>,
    rx: Option<mpsc::Receiver<AvatarResult>>,
}

enum CacheEntry {
    Loading,
    Ready(egui::TextureHandle),
    Failed,
}

struct AvatarResult {
    url: String,
    image: Result<egui::ColorImage, String>,
}

impl AvatarCache {
    pub fn poll(&mut self, ctx: &egui::Context) {
        let Some(rx) = &self.rx else {
            return;
        };

        while let Ok(result) = rx.try_recv() {
            let entry = match result.image {
                Ok(image) => CacheEntry::Ready(ctx.load_texture(
                    format!("author-avatar:{}", result.url),
                    image,
                    egui::TextureOptions::LINEAR,
                )),
                Err(_) => CacheEntry::Failed,
            };
            self.entries.insert(result.url, entry);
        }
    }

    fn texture(&mut self, ctx: &egui::Context, url: &str) -> Option<egui::TextureHandle> {
        if !self.entries.contains_key(url) {
            self.start_fetch(ctx.clone(), url.to_owned());
        }

        match self.entries.get(url) {
            Some(CacheEntry::Ready(texture)) => Some(texture.clone()),
            Some(CacheEntry::Loading | CacheEntry::Failed) | None => None,
        }
    }

    fn start_fetch(&mut self, ctx: egui::Context, url: String) {
        self.entries.insert(url.clone(), CacheEntry::Loading);
        let tx = self.sender().clone();
        std::thread::spawn(move || {
            let image = download_avatar(&url);
            let _ = tx.send(AvatarResult { url, image });
            ctx.request_repaint();
        });
    }

    fn sender(&mut self) -> &mpsc::Sender<AvatarResult> {
        if self.tx.is_none() || self.rx.is_none() {
            let (tx, rx) = mpsc::channel();
            self.tx = Some(tx);
            self.rx = Some(rx);
        }
        self.tx.as_ref().expect("avatar sender initialized")
    }
}

pub fn show(
    ui: &mut egui::Ui,
    cache: &mut AvatarCache,
    avatar_url: Option<&str>,
    login: Option<&str>,
) -> egui::Response {
    show_sized(ui, cache, avatar_url, login, size_for_ui(ui))
}

pub fn show_sized(
    ui: &mut egui::Ui,
    cache: &mut AvatarCache,
    avatar_url: Option<&str>,
    login: Option<&str>,
    size: f32,
) -> egui::Response {
    let desired_size = egui::vec2(size, size);

    if let Some(url) = avatar_url {
        if let Some(texture) = cache.texture(ui.ctx(), url) {
            let corner_radius = egui::CornerRadius::same((size / 2.0).round() as u8);
            return ui.add(
                egui::Image::from_texture(&texture)
                    .fit_to_exact_size(desired_size)
                    .corner_radius(corner_radius),
            );
        }
    }

    placeholder(ui, login, size)
}

pub fn size_for_ui(ui: &egui::Ui) -> f32 {
    let font_size = egui::TextStyle::Body.resolve(ui.style()).size;
    (font_size * 2.4).clamp(28.0, 44.0)
}

fn placeholder(ui: &mut egui::Ui, login: Option<&str>, size: f32) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
    let visuals = ui.visuals();
    let fill = visuals.widgets.inactive.bg_fill.gamma_multiply(1.1);
    let stroke = visuals.widgets.noninteractive.bg_stroke;
    let text = login
        .and_then(|value| value.chars().next())
        .map(|ch| ch.to_uppercase().to_string())
        .unwrap_or_else(|| "?".to_owned());

    ui.painter()
        .circle(rect.center(), rect.width() * 0.5, fill, stroke);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        text,
        egui::FontId::proportional(size * 0.55),
        visuals.widgets.noninteractive.fg_stroke.color,
    );
    response
}

fn download_avatar(url: &str) -> Result<egui::ColorImage, String> {
    let mut response = ureq::get(url)
        .header("User-Agent", USER_AGENT)
        .call()
        .map_err(|error| error.to_string())?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!(
            "avatar request failed with status {}",
            status.as_u16()
        ));
    }
    let bytes = response
        .body_mut()
        .read_to_vec()
        .map_err(|error| error.to_string())?;
    decode_avatar(&bytes)
}

fn decode_avatar(bytes: &[u8]) -> Result<egui::ColorImage, String> {
    let image = image::load_from_memory(bytes).map_err(|error| error.to_string())?;
    let rgba = image.thumbnail(FETCH_SIZE_PX, FETCH_SIZE_PX).to_rgba8();
    let [width, height] = [rgba.width() as usize, rgba.height() as usize];
    let pixels = rgba.into_raw();
    Ok(egui::ColorImage::from_rgba_unmultiplied(
        [width, height],
        &pixels,
    ))
}

#[cfg(test)]
mod tests {
    use super::decode_avatar;

    #[test]
    fn decodes_png_avatar_bytes() {
        let bytes = [
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00,
            0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x44, 0x41, 0x54, 0x78,
            0x9C, 0x63, 0xF8, 0xCF, 0xC0, 0xF0, 0x1F, 0x00, 0x05, 0x00, 0x01, 0xFF, 0x89, 0x99,
            0x3D, 0x1D, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
        ];

        let image = decode_avatar(&bytes).expect("valid avatar");

        assert_eq!(image.size, [64, 64]);
    }
}
