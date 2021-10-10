use eframe::egui;

pub mod fourier_animation;
pub mod svg_preview;

pub trait Window {
    fn name(&self) -> &'static str;

    // Return value: is ui drawn
    fn show(&mut self, ctx: &egui::CtxRef, open: &mut bool) -> bool {
        let mut ui_drawn = false;
        egui::Window::new(self.name())
            .open(open)
            .default_size(egui::vec2(512.0, 256.0))
            .show(ctx, |ui| {
                ui_drawn = true;
                self.ui(ui)
            });
        ui_drawn
    }

    fn ui(&mut self, ui: &mut egui::Ui);
}
