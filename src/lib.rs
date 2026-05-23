use eframe::egui;

pub const APP_TITLE: &str = "Hello World";

#[derive(Default)]
pub struct HelloWorldApp;

impl eframe::App for HelloWorldApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(24.0);
                ui.heading("Hello, world!");
            });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_title_is_hello_world() {
        assert_eq!(APP_TITLE, "Hello World");
    }

    #[test]
    fn app_can_be_constructed() {
        let _app = HelloWorldApp;
    }
}
