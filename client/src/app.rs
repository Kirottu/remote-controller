use reqwest::blocking as rwb;
use serde::{Deserialize, Serialize};
#[cfg(not(target_os = "android"))]
use std::env;
use std::{
    fs,
    path::PathBuf,
    time::{Duration, Instant},
};

enum State {
    Settings,
    MainScreen,
}

#[derive(Serialize, Deserialize, Default, Clone)]
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
        let mut data_path = ndk_glue::native_activity()
            .internal_data_path()
            .to_path_buf();
        #[cfg(not(target_os = "android"))]
        let mut data_path = PathBuf::from(format!(
            "{}/.config/remote-controller",
            env::var("HOME").unwrap()
        ));

        #[cfg(not(target_os = "android"))]
        fs::create_dir_all(&data_path).unwrap();

        data_path.push("config.ron");
        data_path
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

pub struct App {
    state: State,
    config: Config,
    client: rwb::Client,
    notifications: Vec<Notification>,
}

impl App {
    pub fn new() -> Self {
        Self {
            state: State::MainScreen,
            config: Config::load(),
            client: rwb::Client::new(),
            notifications: Vec::new(),
        }
    }

    pub fn update(&mut self, ctx: &egui::Context) {
        self.notifications.retain(|notification| {
            !(notification.to_delete
                || match notification.duration {
                    Some(duration) => notification.instant.elapsed() >= duration,
                    None => false,
                })
        });

        let availabel_rect = ctx.available_rect();

        let main_panel_width = (availabel_rect.width() / 3.0) * 2.0;
        match self.state {
            State::MainScreen => {
                egui::SidePanel::left("left-panel")
                    .default_width(main_panel_width)
                    .min_width(main_panel_width)
                    .max_width(main_panel_width)
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.with_layout(
                            egui::Layout::centered_and_justified(egui::Direction::TopDown),
                            |ui| {
                                if ui.button("PÄÄLLE SAATANA").clicked() {
                                    match self
                                        .client
                                        .post(format!("{}/turn_on", self.config.address))
                                        .send()
                                    {
                                        Ok(_) => self.notifications.push(Notification::new(
                                            "Success sending request".to_string(),
                                            Severity::Info,
                                            Some(Duration::from_secs(2)),
                                        )),
                                        Err(why) => self.notifications.push(Notification::new(
                                            format!("Error sending request:\n{}", why),
                                            Severity::Error,
                                            None,
                                        )),
                                    };
                                }
                            },
                        );
                    });
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.with_layout(
                        egui::Layout::centered_and_justified(egui::Direction::TopDown),
                        |ui| {
                            if ui.button("⚙").clicked() {
                                self.state = State::Settings;
                            }
                        },
                    );
                });
            }
            State::Settings => {
                egui::SidePanel::left("left-panel")
                    .default_width(main_panel_width)
                    .min_width(main_panel_width)
                    .max_width(main_panel_width)
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.label("Address");
                        ui.monospace(&self.config.address);
                        egui::Grid::new("keypad").num_columns(3).show(ui, |ui| {
                            for row in &[
                                &["1", "2", "3"],
                                &["4", "5", "6"],
                                &["7", "8", "9"],
                                &[".", "0", ":"],
                                &["h", "t", "p"],
                                &["s", "/", "<"],
                            ] {
                                ui.end_row();
                                for chr in *row {
                                    if ui
                                        .button(
                                            egui::RichText::new(&format!("  {}  ", *chr))
                                                .monospace(),
                                        )
                                        .clicked()
                                    {
                                        if *chr == "<" {
                                            self.config.address.pop();
                                        } else {
                                            self.config.address.push_str(chr);
                                        }
                                    }
                                }
                            }
                        });
                        /*
                        let response = ui.text_edit_multiline(&mut self.config.address);
                        if response.gained_focus() {
                            #[cfg(target_os = "android")]
                            {
                                let ctx = ndk_glue::native_activity();

                                let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }.unwrap();
                                let env = vm.attach_current_thread().unwrap();

                                let class_ctxt = env.find_class("android/content/Context").unwrap();
                                let ime = env
                                    .get_static_field(
                                        class_ctxt,
                                        "INPUT_METHOD_SERVICE",
                                        "Ljava/lang/String;",
                                    )
                                    .unwrap();
                                let ime_manager = env
                                    .call_method(
                                        unsafe { jni::objects::JObject::from_raw(ctx.activity()) },
                                        "getSystemService",
                                        "(Ljava/lang/String;)Ljava/lang/Object;",
                                        &[ime],
                                    )
                                    .unwrap()
                                    .l()
                                    .unwrap();

                                let jni_window = env
                                    .call_method(
                                        unsafe { jni::objects::JObject::from_raw(ctx.activity()) },
                                        "getWindow",
                                        "()Landroid/view/Window;",
                                        &[],
                                    )
                                    .unwrap()
                                    .l()
                                    .unwrap();
                                let view = env
                                    .call_method(
                                        jni_window,
                                        "getDecorView",
                                        "()Landroid/view/View;",
                                        &[],
                                    )
                                    .unwrap()
                                    .l()
                                    .unwrap();

                                let _result = env
                                    .call_method(
                                        ime_manager,
                                        "showSoftInput",
                                        "(Landroid/view/View;I)Z",
                                        &[view.into(), 0i32.into()],
                                    )
                                    .unwrap()
                                    .z()
                                    .unwrap();
                            }
                        }*/
                    });
                egui::CentralPanel::default().show(ctx, |ui| {
                    let height = ui.available_height() / 2.0;
                    egui::Grid::new("settings-buttons")
                        .num_columns(1)
                        .min_row_height(height)
                        .show(ui, |ui| {
                            ui.with_layout(
                                egui::Layout::centered_and_justified(egui::Direction::TopDown),
                                |ui| {
                                    if ui.button("CLOSE").clicked() {
                                        self.state = State::MainScreen;
                                    }
                                },
                            );
                            ui.end_row();
                            ui.with_layout(
                                egui::Layout::centered_and_justified(egui::Direction::TopDown),
                                |ui| {
                                    if ui.button("SAVE").clicked() {
                                        self.config.save();
                                        self.notifications.push(Notification::new(
                                            "Successfully saved".to_string(),
                                            Severity::Info,
                                            Some(Duration::from_secs(2)),
                                        ));
                                    }
                                },
                            );
                        });
                });
            }
        }
        let mut height_offset = 10.0;
        for notification in &mut self.notifications {
            let response = egui::Area::new(notification.instant)
                .order(egui::Order::Foreground)
                .fixed_pos(egui::pos2(10.0, height_offset))
                .show(ctx, |ui| {
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
    }
}
