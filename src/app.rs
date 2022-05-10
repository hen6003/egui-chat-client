use crate::net::{client, commands::ChatCommands};

use egui::vec2;
use std::sync::{Arc, Mutex};
use std::thread;
use tokio::sync::mpsc;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
//#[derive(serde::Deserialize, serde::Serialize)]
//#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct Client {
    //#[serde(skip)]
    messages: Arc<Mutex<Vec<ChatCommands>>>,

    //#[serde(skip)]
    network_send: mpsc::Sender<String>,

    //#[serde(skip)]
    message: String,
}

impl Client {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Start network thread
        let messages = Arc::new(Mutex::new(Vec::new()));
        let messages_network = Arc::clone(&messages);
        let (network_send, network_recv) = mpsc::channel::<String>(100);
        let egui_ctx = cc.egui_ctx.clone();

        tokio::spawn(
            async move { client::network(messages_network, network_recv, egui_ctx).await },
        );

        //tokio::spawn(async move {
        //    let mut lock = messages_network.lock().unwrap();
        //    lock.push(ChatCommands::Message {
        //        sender: "hello".to_string(),
        //        message: "world".to_string(),
        //    });
        //    lock.push(ChatCommands::Message {
        //        sender: "longer_name".to_string(),
        //        message: "This is a message".to_string(),
        //    });
        //});

        // This is also where you can customized the look at feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        //if let Some(storage) = cc.storage {
        //    eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        //} else {
        Self {
            messages,
            network_send,
            message: String::new(),
        }
        //}
    }
}

impl eframe::App for Client {
    /// Called by the frame work to save state before shutdown.
    //fn save(&mut self, storage: &mut dyn eframe::Storage) {
    //    eframe::set_value(storage, eframe::APP_KEY, self);
    //}

    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.allocate_ui(
                vec2(ui.available_width(), ui.available_height() - 20.0),
                |ui| {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .stick_to_bottom()
                        .show(ui, |ui| {
                            egui::Grid::new("my_grid").num_columns(2).show(ui, |ui| {
                                let lock = self.messages.lock().unwrap();

                                for row in 0..lock.len() {
                                    let c = &lock[row];

                                    for col in 0..2 {
                                        if col == 0 {
                                            ui.with_layout(egui::Layout::right_to_left(), |ui| {
                                                match c {
                                                    ChatCommands::Message {
                                                        sender,
                                                        message: _,
                                                    } => ui.heading(sender),
                                                };
                                            });
                                        } else {
                                            match c {
                                                ChatCommands::Message { sender: _, message } => {
                                                    ui.label(message)
                                                }
                                            };
                                        }
                                    }
                                    ui.end_row();
                                }
                            });
                        });
                },
            );

            let response = ui.add(
                egui::TextEdit::singleline(&mut self.message)
                    .desired_width(f32::INFINITY)
                    .hint_text("Enter message..."),
            );

            if response.lost_focus() && ui.input().key_pressed(egui::Key::Enter) {
                let message = self.message.clone();

                self.message.clear();
                response.request_focus();

                // Start new thread to send message to avoid blocking ui draw
                let sender = self.network_send.clone();
                thread::spawn(move || {
                    sender.blocking_send(message).unwrap();
                });
            }
        });
    }
}
