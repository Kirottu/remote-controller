use std::{
    collections::VecDeque,
    env, fs,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use egui::RichText;
use egui_miniquad as egui_mq;
use miniquad as mq;
use reqwest::blocking as rwb;
use serde::{Deserialize, Serialize};

enum State {
    Settings,
    MainScreen,
}

#[derive(Serialize, Deserialize, Default)]
struct Config {
    address: String,
}

impl Config {
    fn load() -> Self {
        let path = Self::get_path();
        if path.exists() {
            ron::from_str(&fs::read_to_string(path).unwrap()).unwrap()
        } else {
            Config::default()
        }
    }
    fn save(&self) {
        let path = Self::get_path();

        fs::write(path, ron::to_string(self).unwrap()).unwrap();
    }

    fn get_path() -> PathBuf {
        #[cfg(target_os = "android")]
        let data_path = ndk_glue::native_activity().internal_data_path().to_string();
        #[cfg(not(target_os = "android"))]
        let data_path = format!("{}/.config/remote-controller", env::var("HOME").unwrap());

        #[cfg(not(target_os = "android"))]
        fs::create_dir_all(&data_path).unwrap();

        PathBuf::from(format!("{}/config.ron", data_path))
    }
}

enum Severity {
    Error,
    Info,
}

struct Notification {
    pub content: String,
    pub severity: Severity,
    pub instant: Instant,
    pub duration: Option<Duration>,
    pub to_delete: bool,
}

impl Notification {
    fn new(content: String, severity: Severity, duration: Option<Duration>) -> Self {
        Self {
            content,
            severity,
            instant: Instant::now(),
            duration,
            to_delete: false,
        }
    }
}

struct App {
    egui_mq: egui_mq::EguiMq,
    state: State,
    config: Config,
    client: rwb::Client,
    notifications: Vec<Notification>,
}

impl App {
    fn new(ctx: &mut mq::Context) -> Self {
        Self {
            egui_mq: egui_mq::EguiMq::new(ctx),
            state: State::MainScreen,
            config: Config::load(),
            client: rwb::Client::new(),
            notifications: Vec::new(),
        }
    }
}

impl mq::EventHandler for App {
    fn update(&mut self, _ctx: &mut mq::Context) {
        self.notifications.retain(|notification| {
            !(notification.to_delete
                || match notification.duration {
                    Some(duration) => notification.instant.elapsed() >= duration,
                    None => false,
                })
        });
    }

    fn draw(&mut self, mq_ctx: &mut mq::Context) {
        mq_ctx.clear(Some((1.0, 1.0, 1.0, 1.0)), None, None);
        mq_ctx.begin_default_pass(mq::PassAction::clear_color(0.0, 0.0, 0.0, 1.0));
        mq_ctx.end_render_pass();

        // We create this here as we cannot directly add to the main variable inside the egui draw loop
        let mut new_notifications = Vec::new();

        self.egui_mq.run(mq_ctx, |_mq_ctx, egui_ctx| {
            egui_ctx.set_pixels_per_point(4.0);
            match self.state {
                State::MainScreen => {
                    egui::TopBottomPanel::bottom("bottom-panel").show(egui_ctx, |ui| {
                        ui.with_layout(
                            egui::Layout::centered_and_justified(egui::Direction::TopDown),
                            |ui| {
                                if ui.button("SETTINGS").clicked() {
                                    self.state = State::Settings;
                                }
                            },
                        );
                    });
                    egui::CentralPanel::default().show(egui_ctx, |ui| {
                        ui.with_layout(
                            egui::Layout::centered_and_justified(egui::Direction::TopDown),
                            |ui| {
                                if ui.button("PÄÄLLE SAATANA").clicked() {
                                    match self
                                        .client
                                        .post(format!("{}/turn_on", self.config.address))
                                        .send()
                                    {
                                        Ok(_) => new_notifications.push(Notification::new(
                                            "Success sending request".to_string(),
                                            Severity::Info,
                                            None,
                                        )),
                                        Err(why) => new_notifications.push(Notification::new(
                                            format!("Error sending request:\n{}", why),
                                            Severity::Error,
                                            None,
                                        )),
                                    };
                                }
                            },
                        );
                    });
                }
                State::Settings => {
                    egui::CentralPanel::default().show(egui_ctx, |ui| {});
                    egui::TopBottomPanel::bottom("bottom-panel").show(egui_ctx, |ui| {
                        ui.columns(2, |ui| {
                            if ui[0].button("SAVE").clicked() {
                                self.config.save();
                                new_notifications.push(Notification::new(
                                    "Successfully saved".to_string(),
                                    Severity::Info,
                                    Some(Duration::from_secs(2)),
                                ));
                            }
                            if ui[1].button("CLOSE").clicked() {
                                self.state = State::MainScreen;
                            }
                        });
                    });
                }
            }
            let mut height_offset = 10.0;
            for notification in &mut self.notifications {
                let response = egui::Area::new(notification.instant)
                    .order(egui::Order::Foreground)
                    .fixed_pos(egui::pos2(10.0, height_offset))
                    .show(egui_ctx, |ui| {
                        egui::Frame::default()
                            .fill(match notification.severity {
                                Severity::Error => egui::Color32::RED,
                                Severity::Info => egui::Color32::GREEN,
                            })
                            .rounding(egui::Rounding::same(2.0))
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new(&notification.content)
                                        .color(egui::Color32::WHITE),
                                )
                            })
                    })
                    .response;
                notification.to_delete = response.clicked();
                height_offset += response.rect.height() + 10.0;
            }
        });
        self.egui_mq.draw(mq_ctx);

        self.notifications.extend(new_notifications);
    }

    fn mouse_motion_event(&mut self, _ctx: &mut mq::Context, _x: f32, _y: f32) {
        self.egui_mq.mouse_motion_event(_x, _y);
    }

    fn mouse_button_down_event(
        &mut self,
        _ctx: &mut mq::Context,
        _button: mq::MouseButton,
        _x: f32,
        _y: f32,
    ) {
        self.egui_mq.mouse_button_down_event(_ctx, _button, _x, _y);
    }

    fn mouse_button_up_event(
        &mut self,
        _ctx: &mut mq::Context,
        _button: mq::MouseButton,
        _x: f32,
        _y: f32,
    ) {
        self.egui_mq.mouse_button_up_event(_ctx, _button, _x, _y);
    }

    fn touch_event(
        &mut self,
        ctx: &mut mq::Context,
        phase: mq::TouchPhase,
        _id: u64,
        x: f32,
        y: f32,
    ) {
        if phase == mq::TouchPhase::Started {
            self.mouse_button_down_event(ctx, mq::MouseButton::Left, x, y);
        }

        if phase == mq::TouchPhase::Ended {
            self.mouse_button_up_event(ctx, mq::MouseButton::Left, x, y);
        }

        if phase == mq::TouchPhase::Moved {
            self.mouse_motion_event(ctx, x, y);
        }
    }
}

fn main() {
    let conf = mq::conf::Conf {
        ..Default::default()
    };
    mq::start(conf, |mut ctx| Box::new(App::new(&mut ctx)));
}
