use eframe::egui;
use egui::plot::{Line, Plot, Value, Values};
use num::complex::Complex;
use std::time::Instant;

type SvgFnType = dyn Fn(f64) -> Complex<f64>;

pub struct SvgPreviewWindow {
    pub svg_fn: Option<Box<SvgFnType>>,
    animate_start_t: Option<Instant>,
    // Progress per second
    animate_speed: f64,
    t: f64,
}

impl Default for SvgPreviewWindow {
    fn default() -> Self {
        Self {
            svg_fn: None,
            animate_start_t: None,
            animate_speed: 0.23,
            t: 0.0,
        }
    }
}

impl super::Window for SvgPreviewWindow {
    fn name(&self) -> &'static str {
        "SVG Preview"
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        let Self {
            svg_fn,
            animate_start_t,
            animate_speed,
            t,
        } = self;

        let mut local_t = if let Some(instant) = animate_start_t {
            (*t + instant.elapsed().as_secs_f64() * *animate_speed).fract()
        } else {
            *t
        };

        if let Some(func) = svg_fn {
            ui.horizontal(|ui| {
                let mut animation_should_stop = false;
                let animation_running = animate_start_t.is_some();
                let slider = egui::Slider::new(&mut local_t, 0.0..=1.0).clamp_to_range(true);
                ui.label("Input of t:");

                if ui.add(slider).changed() {
                    *animate_start_t = None;
                    animation_should_stop = true;
                }

                let control_btn_text = if animation_running { "⏸" } else { "▶" };
                if ui.button(control_btn_text).clicked() {
                    if animation_running {
                        *animate_start_t = None;
                        animation_should_stop = true;
                    } else {
                        *animate_start_t = Some(Instant::now());
                    }
                }

                // Flush t where necessary
                if animation_should_stop {
                    *t = local_t;
                }
            });

            ui.label(format!("Output: {:.6}", func(local_t)));

            const ITERATE_COUNT: usize = 1000;
            let values_iter = (0..=ITERATE_COUNT).map(|i| {
                let t = i as f64 / ITERATE_COUNT as f64 * local_t;
                let result = func(t);
                Value::new(result.re, result.im)
            });
            let line = Line::new(Values::from_values_iter(values_iter));
            ui.add(Plot::new("svg_plot").line(line).data_aspect(1.0));
        } else {
            ui.label("Error: SVG is invalid or not set.");
        }
    }
}

impl SvgPreviewWindow {
    pub fn reset(&mut self) {
        self.svg_fn = None;
        self.animate_start_t = None;
        self.t = 0.0;
    }

    pub fn set(&mut self, svg_fn: Option<Box<SvgFnType>>) {
        self.svg_fn = svg_fn;
    }

    pub fn set_speed(&mut self, speed: f64) {
        self.animate_speed = speed;
    }

    pub fn play(&mut self) {
        self.animate_start_t = Some(Instant::now());
    }

    pub fn pause(&mut self) {
        if let Some(instant) = self.animate_start_t {
            // Flush of t is necessary
            self.t = (self.t + instant.elapsed().as_secs_f64() * self.animate_speed).fract();
        }
        self.animate_start_t = None;
    }

    pub fn is_playing(&self) -> bool {
        self.animate_start_t.is_some()
    }
}
