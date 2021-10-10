use crate::util::math::FourierSeriesDesc;
use eframe::egui::{self, plot::Arrows};
use egui::plot::{Line, Plot, Value, Values};
use num::complex::Complex;
use std::{cmp::Ordering, iter, time::Instant};

pub struct FourierAnimationWindow {
    series_desc: Option<FourierSeriesDesc<f64>>,
    animate_start_t: Option<Instant>,
    // Progress per second
    animate_speed: f64,
    t: f64,
}

impl Default for FourierAnimationWindow {
    fn default() -> Self {
        FourierAnimationWindow {
            series_desc: None,
            animate_start_t: None,
            animate_speed: 0.2,
            t: 0.0,
        }
    }
}

impl super::Window for FourierAnimationWindow {
    fn name(&self) -> &'static str {
        "Fourier Animation"
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        let Self {
            series_desc,
            animate_start_t,
            animate_speed,
            t,
        } = self;

        let mut local_t = if let Some(instant) = animate_start_t {
            (*t + instant.elapsed().as_secs_f64() * *animate_speed).fract()
        } else {
            *t
        };

        if let Some(desc) = series_desc {
            let func = desc.as_fn();

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
            let lines_iter = (0..=ITERATE_COUNT).map(|i| {
                let t = i as f64 / ITERATE_COUNT as f64 * local_t;
                let result = func(t);
                Value::new(result.re, result.im)
            });
            let line = Line::new(Values::from_values_iter(lines_iter));
            // let arrow_origins_iter = (0..=10).map(|i| {
            //     Value::new(0.0, 0.0)
            // });
            // let arrow_tips_iter = (0..=10).map(|i| {
            //     let t = i as f64 / 10 as f64 * local_t;
            //     let result = func(t);
            //     Value::new(result.re, result.im)
            // });
            let coefficients_n = desc.as_vec().len();
            let half_range = ((coefficients_n - 1) / 2) as isize;
            let mut coefficients: Vec<_> = desc
                .as_vec()
                .iter()
                .enumerate()
                .map(|(a, b)| (a as isize - half_range, b))
                .collect();
            coefficients.sort_by(|&(ida, _), &(idb, _)| {
                if ida.abs() < idb.abs() {
                    Ordering::Less
                } else if ida.abs() > idb.abs() {
                    Ordering::Greater
                } else if ida > idb {
                    Ordering::Less
                } else {
                    Ordering::Equal
                }
            });
            let arrows_pre_sum: Vec<_> = coefficients
                .iter()
                .map(|x| {
                    *x.1 * Complex::new(0.0, local_t * x.0 as f64 * 2.0 * std::f64::consts::PI)
                        .exp()
                })
                .scan(Complex::new(0.0, 0.0), |state, x| {
                    *state += x;
                    Some(Value::new(state.re, state.im))
                })
                .collect();
            let arrow = Arrows::new(
                Values::from_values_iter(
                    iter::once(Value::new(0.0, 0.0)).chain(arrows_pre_sum.iter().cloned()),
                ),
                Values::from_values_iter(arrows_pre_sum.iter().cloned()),
            );
            ui.add(
                Plot::new("fourier_plot")
                    .line(line)
                    .arrows(arrow)
                    .data_aspect(1.0),
            );
        } else {
            ui.label("Error: Fourier series data is invalid or not set.");
        }
    }
}

impl FourierAnimationWindow {
    pub fn reset(&mut self) {
        self.series_desc = None;
        self.animate_start_t = None;
        self.t = 0.0;
    }

    pub fn set_speed(&mut self, speed: f64) {
        self.animate_speed = speed;
    }

    pub fn set(&mut self, desc: Option<FourierSeriesDesc<f64>>) {
        self.series_desc = desc;
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
