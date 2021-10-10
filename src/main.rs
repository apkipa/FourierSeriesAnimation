#![windows_subsystem = "windows"]

use num::complex::Complex;
use std::{
    convert::{TryFrom, TryInto},
    ops::{Deref, DerefMut},
    vec,
};

use eframe::{egui, epi};

mod ui;
mod util;

use ui::{
    frame_history::FrameHistory,
    svg_select::SvgSelect,
    window::{fourier_animation::FourierAnimationWindow, svg_preview::SvgPreviewWindow, Window},
};

struct WindowDesc<T: ui::window::Window> {
    is_open: bool,
    window: T,
}

impl<T: Window> Deref for WindowDesc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.window
    }
}

impl<T: Window> DerefMut for WindowDesc<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.window
    }
}

impl<T: Window + Default> Default for WindowDesc<T> {
    fn default() -> Self {
        Self {
            is_open: false,
            window: Default::default(),
        }
    }
}

impl<T: Window> WindowDesc<T> {
    fn show(&mut self, ctx: &egui::CtxRef) -> bool {
        self.window.show(ctx, &mut self.is_open)
    }
}

struct MyApp {
    frame_history: FrameHistory,
    animation_window: WindowDesc<FourierAnimationWindow>,
    svg_select: SvgSelect,
    svg_preview_window: WindowDesc<SvgPreviewWindow>,
    fourier_series_n: usize,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            frame_history: Default::default(),
            animation_window: Default::default(),
            svg_select: Default::default(),
            svg_preview_window: Default::default(),
            fourier_series_n: 11,
        }
    }
}

fn cubic_bezier(
    p0: Complex<f64>,
    p1: Complex<f64>,
    p2: Complex<f64>,
    p3: Complex<f64>,
    t: f64,
) -> Complex<f64> {
    let inv_t = 1.0 - t;
    inv_t.powi(3) * p0
        + 3.0 * inv_t.powi(2) * t * p1
        + 3.0 * inv_t * t.powi(2) * p2
        + t.powi(3) * p3
}

#[derive(Debug)]
enum CmdData {
    Move(Complex<f64>),
    CubicCurve(Complex<f64>, Complex<f64>, Complex<f64>),
}

#[derive(thiserror::Error, Debug)]
enum TryFromCommandError {
    #[error("Found unrecognized command `{0:?}`")]
    UnrecognizedCommand(String),
    #[error("Parameters is invalid")]
    InvalidParameter,
}

struct VecCmdData(Vec<CmdData>);

impl TryFrom<&svg::node::element::path::Command> for VecCmdData {
    type Error = TryFromCommandError;

    fn try_from(value: &svg::node::element::path::Command) -> Result<Self, Self::Error> {
        use svg::node::element::path::{Command, Position::Absolute};

        let result = match value {
            Command::Move(Absolute, param) => {
                if param.len() != 2 {
                    return Err(Self::Error::InvalidParameter);
                }

                vec![CmdData::Move(Complex::new(
                    param[0].into(),
                    param[1].into(),
                ))]
            }
            Command::CubicCurve(Absolute, param) => {
                if param.len() % 6 != 0 {
                    return Err(Self::Error::InvalidParameter);
                }

                let mut vec_result = Vec::new();
                for s in param.chunks_exact(6) {
                    let p1 = Complex::new(s[0].into(), s[1].into());
                    let p2 = Complex::new(s[2].into(), s[3].into());
                    let p3 = Complex::new(s[4].into(), s[5].into());
                    vec_result.push(CmdData::CubicCurve(p1, p2, p3));
                }

                vec_result
            }
            Command::Close => vec![],
            other_cmd => return Err(Self::Error::UnrecognizedCommand(format!("{:?}", other_cmd))),
        };

        Ok(VecCmdData(result))
    }
}

fn parse_svg_into_proc<T: AsRef<std::path::Path>>(
    path: T,
) -> Option<Box<dyn Fn(f64) -> Complex<f64>>> {
    use svg::node::element::path::Data;
    use svg::node::element::tag::Path;
    use svg::parser::Event;

    let mut content = String::new();

    let mut cmd_vec: Vec<CmdData> = Vec::new();
    let mut segments_count: usize = 0;

    for event in svg::open(path, &mut content).unwrap() {
        match event {
            Event::Tag(Path, _, attributes) => {
                let data = attributes.get("d")?;
                let data = Data::parse(data).ok()?;
                for command in data.iter() {
                    match command.try_into() {
                        Ok(data) => {
                            let mut data: VecCmdData = data;
                            cmd_vec.append(&mut data.0);
                        }
                        Err(e) => {
                            eprintln!("SVG parse error: {}", e);
                            return None;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    for i in &cmd_vec {
        if let CmdData::Move(..) = i {
            // Move is not considered a segment
        } else {
            segments_count += 1;
        }
    }

    // println!("Parsed SVG: {:#?}", cmd_vec);
    // println!("Total {} segment(s).", segments_count);

    let func = move |t| {
        let idx_prog = t * segments_count as f64;
        let idx = idx_prog as usize;
        let prog = idx_prog - idx as f64;

        let mut cur_pos = Complex::new(0.0, 0.0);
        let mut cur_idx = 0;
        for cmd in &cmd_vec {
            match cmd {
                CmdData::Move(p0) => {
                    cur_pos = *p0;
                }
                CmdData::CubicCurve(p1, p2, p3) => {
                    cur_idx += 1;
                    if cur_idx > idx {
                        return cubic_bezier(cur_pos, *p1, *p2, *p3, prog);
                    }
                    cur_pos = *p3;
                }
            }
        }

        cur_pos
    };

    Some(Box::new(func))
}

impl epi::App for MyApp {
    fn name(&self) -> &'static str {
        "Fourier Series Drawing Animation"
    }

    fn update(&mut self, ctx: &egui::CtxRef, frame: &mut epi::Frame<'_>) {
        let Self {
            frame_history,
            animation_window,
            svg_select,
            svg_preview_window,
            fourier_series_n,
        } = self;

        frame_history.on_new_frame(ctx.input().time, frame.info().cpu_usage);

        if let Some(pixels_per_point) = frame.info().native_pixels_per_point {
            ctx.set_pixels_per_point(pixels_per_point * 1.2);
        }

        if let [file, ..] = &ctx.input().raw.dropped_files[..] {
            let path = file.path.as_ref();
            if path
                .map(|p| p.extension())
                .flatten()
                .map_or(false, |s| s == "svg")
            {
                svg_select.disp_path = path.map(|p| p.display().to_string());
            }
        }

        if !ctx.input().raw.hovered_files.is_empty() {
            use egui::{Align2, Color32, Id, LayerId, Order, TextStyle};

            let painter = ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("file_drop")));
            let screen_rect = ctx.input().screen_rect();
            painter.rect_filled(screen_rect, 0.0, Color32::from_black_alpha(192));
            painter.text(
                screen_rect.center(),
                Align2::CENTER_CENTER,
                "Drop files here",
                TextStyle::Heading,
                Color32::WHITE,
            );
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("This application helps you calculate fourier series functions from svgs.");
            svg_select.ui(ui);
            ui.scope(|ui| {
                // let should_btn_enable = svg_select.disp_path.is_some();
                let btn_msg = "Preview SVG";
                if let Some(path) = &svg_select.disp_path {
                    if ui.button(btn_msg).clicked() {
                        svg_preview_window.reset();
                        svg_preview_window.is_open = true;
                        svg_preview_window.set(parse_svg_into_proc(path));
                        svg_preview_window.play();
                    }
                } else {
                    ui.set_enabled(false);
                    if ui.button(btn_msg).clicked() {
                        unreachable!("Button should not be clicked at this time.");
                    }
                }
            });

            ui.separator();

            ui.label("Note: n must be an odd number for series to be correctly calculated!");
            let slider_n = egui::Slider::new(fourier_series_n, 9..=501).clamp_to_range(true);
            ui.add(slider_n);

            ui.scope(|ui| {
                // ui.set_enabled(svg_select.disp_path.is_some());
                // if ui.button("Calculate & Show").clicked() {
                //     animation_window.is_open = true;
                // }

                let btn_msg = "Calculate & Show";
                if let Some(path) = &svg_select.disp_path {
                    if ui.button(btn_msg).clicked() {
                        animation_window.reset();
                        animation_window.is_open = true;

                        if *fourier_series_n % 2 == 0 {
                            *fourier_series_n += 1;
                        }

                        let desc = parse_svg_into_proc(path).map(|proc| {
                            util::math::convert_to_fourier_series(proc, *fourier_series_n)
                        });
                        // dbg!(&desc);
                        animation_window.set(desc);
                        animation_window.play();
                    }
                } else {
                    ui.set_enabled(false);
                    if ui.button(btn_msg).clicked() {
                        unreachable!("Button should not be clicked at this time.");
                    }
                }
            });

            ui.separator();

            frame_history.ui(ui);

            ui.separator();

            ui.horizontal(|ui| {
                use egui::special_emojis::GITHUB;
                ui.label("ℹ Powered by");
                ui.hyperlink_to(
                    format!("{} egui", GITHUB),
                    "https://www.github.com/emilk/egui",
                );
            });
        });

        let mut drawn = animation_window.show(ctx) && animation_window.is_playing();
        drawn = (svg_preview_window.show(ctx) && svg_preview_window.is_playing()) || drawn;

        if drawn {
            ctx.request_repaint();
        }
    }
}

fn main() {
    let options = eframe::NativeOptions {
        drag_and_drop_support: true,
        ..Default::default()
    };
    eframe::run_native(Box::new(MyApp::default()), options);
    // eframe::run_native(Box::new(egui_demo_lib::WrapApp::default()), options);
}
