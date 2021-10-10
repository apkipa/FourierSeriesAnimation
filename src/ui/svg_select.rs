use eframe::egui;

pub struct SvgSelect {
    pub disp_path: Option<String>,
}

impl Default for SvgSelect {
    fn default() -> Self {
        Self { disp_path: None }
    }
}

impl SvgSelect {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.label("Try dragging a svg into the window.");
        if let Some(path) = &self.disp_path {
            ui.label(format!("Selected svg: {}", path));
        } else {
            ui.label("No svg is selected.");
        }
    }
}
