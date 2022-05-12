use crate::net::{client, commands::ChatCommands, connection::ConnectionData};

use egui::vec2;
use std::sync::{Arc, Mutex};
use std::thread;
use tokio::sync::mpsc;

struct Tab {
    messages: Arc<Mutex<Vec<ChatCommands>>>,
    network_send: mpsc::Sender<String>,
    message: String,

    connection: ConnectionData,
}

impl Tab {
    fn new(egui_ctx: egui::Context, connection: ConnectionData) -> Self {
        let messages = Arc::new(Mutex::new(Vec::new()));
        let messages_network = Arc::clone(&messages);
        let (network_send, network_recv) = mpsc::channel::<String>(100);

        {
            let connection = connection.clone();

            tokio::spawn(async move {
                client::network(messages_network, network_recv, egui_ctx, connection).await
            });
        }

        Self {
            messages,
            network_send,
            message: String::new(),
            connection,
        }
    }
}

#[derive(Default)]
pub struct Client {
    tabs: Vec<Tab>,

    current_tab: usize,

    show_server_edit: bool,
    server_edit_name: String,
    server_edit_address: String,
}

impl Client {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>, server: Option<&str>) -> Self {
        let mut start_tab = 0;
        let mut connections = if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            Vec::new()
        };

        if let Some(server) = server {
            connections.push(ConnectionData::new(server, "nobody"));
            start_tab = connections.len() - 1;
        }

        if connections.len() == 0 {
            connections.push(ConnectionData::default());
        }

        // Start network thread
        let mut tabs = Vec::new();
        for c in connections {
            let egui_ctx = cc.egui_ctx.clone();
            tabs.push(Tab::new(egui_ctx, c));
        }

        // This is also where you can customized the look at feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        Self {
            tabs,
            current_tab: start_tab,

            ..Default::default()
        }
    }
}

impl eframe::App for Client {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        let mut connections = Vec::new();

        for tab in &self.tabs {
            connections.push(&tab.connection);
        }

        eframe::set_value(storage, eframe::APP_KEY, &connections);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left("left_panel").show(ctx, |ui| {
            egui::warn_if_debug_build(ui);

            let mut to_remove = Vec::new();
            for (i, tab) in self.tabs.iter().enumerate() {
                ui.horizontal(|ui| {
                    if ui.button(tab.connection.server()).clicked() {
                        self.current_tab = i;
                    }

                    if ui
                        .add_enabled(
                            self.tabs.len() != 1,
                            egui::Button::new(egui::RichText::new("❌").color(egui::Color32::RED)),
                        )
                        .clicked()
                    {
                        to_remove.push(i);
                    }
                });
            }
            for i in to_remove {
                self.tabs.remove(i);
            }

            if self.current_tab >= self.tabs.len() {
                self.current_tab = self.tabs.len() - 1;
            }

            if ui
                .button(egui::RichText::new("+").color(egui::Color32::GREEN))
                .clicked()
            {
                self.show_server_edit = true;
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.allocate_ui(
                vec2(ui.available_width(), ui.available_height() - 20.0),
                |ui| {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .stick_to_bottom()
                        .show(ui, |ui| {
                            egui::Grid::new("message_grid")
                                .num_columns(2)
                                .show(ui, |ui| {
                                    let lock = self.tabs[self.current_tab].messages.lock().unwrap();

                                    for row in 0..lock.len() {
                                        let c = &lock[row];

                                        for col in 0..2 {
                                            if col == 0 {
                                                ui.with_layout(
                                                    egui::Layout::right_to_left(),
                                                    |ui| match c {
                                                        ChatCommands::Message {
                                                            sender,
                                                            message: _,
                                                        } => {
                                                            ui.heading(sender);
                                                        }

                                                        ChatCommands::UserConnected { name: _ } => {
                                                            ui.heading(
                                                                egui::RichText::new("+")
                                                                    .color(egui::Color32::GREEN),
                                                            );
                                                        }

                                                        ChatCommands::UserDisconnected {
                                                            name: _,
                                                        } => {
                                                            ui.heading(
                                                                egui::RichText::new("-")
                                                                    .color(egui::Color32::RED),
                                                            );
                                                        }

                                                        _ => {
                                                            ui.heading(
                                                                egui::RichText::new("!").color(
                                                                    egui::Color32::DARK_GREEN,
                                                                ),
                                                            );
                                                        }
                                                    },
                                                );
                                            } else {
                                                match c {
                                                    ChatCommands::Message {
                                                        sender: _,
                                                        message,
                                                    } => {
                                                        ui.label(message);
                                                    }

                                                    ChatCommands::UserConnected { name } => {
                                                        ui.strong(format!("{} connected", name));
                                                    }

                                                    ChatCommands::UserDisconnected { name } => {
                                                        ui.strong(format!("{} disconnected", name));
                                                    }

                                                    ChatCommands::UserRenamed {
                                                        oldname,
                                                        newname,
                                                    } => {
                                                        ui.strong(format!(
                                                            "{} changed names to {}",
                                                            oldname, newname
                                                        ));
                                                    }
                                                }
                                            }
                                        }
                                        ui.end_row();
                                    }
                                });
                        });
                },
            );

            let response = ui.add(
                egui::TextEdit::singleline(&mut self.tabs[self.current_tab].message)
                    .desired_width(f32::INFINITY)
                    .hint_text("Enter message..."),
            );

            if response.lost_focus() && ui.input().key_pressed(egui::Key::Enter) {
                let message = self.tabs[self.current_tab].message.clone();

                self.tabs[self.current_tab].message.clear();
                response.request_focus();

                // Start new thread to send message to avoid blocking ui draw
                let sender = self.tabs[self.current_tab].network_send.clone();
                thread::spawn(move || {
                    sender.blocking_send(message).unwrap();
                });
            }
        });

        if self.show_server_edit {
            egui::Window::new("Server details")
                .fixed_size((200.0, 60.0))
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Server address");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.server_edit_address)
                                .desired_width(f32::INFINITY)
                                .hint_text("example.com"),
                        );
                    });

                    ui.horizontal(|ui| {
                        ui.label("Username");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.server_edit_name)
                                .desired_width(f32::INFINITY)
                                .hint_text("john_smith"),
                        );
                    });

                    ui.with_layout(egui::Layout::right_to_left(), |ui| {
                        if ui.button("Add").clicked() {
                            self.tabs.push(Tab::new(
                                ctx.clone(),
                                ConnectionData::new(
                                    &self.server_edit_address,
                                    &self.server_edit_name,
                                ),
                            ));

                            self.server_edit_address.clear();
                            self.server_edit_name.clear();
                            self.show_server_edit = false;
                        }

                        if ui.button("Cancel").clicked() {
                            self.show_server_edit = false;
                        }
                    });
                });
        }
    }
}
