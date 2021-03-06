use crate::net::{client, commands::*, connection::ConnectionData};

use egui::vec2;
use std::thread;
use tokio::sync::mpsc::{self, error::TryRecvError};

struct Tab {
    messages: Vec<ChatCommands>,
    send: mpsc::Sender<String>,
    recv: mpsc::Receiver<ClientCommands>,
    message: String,

    connect_state: ConnectState,

    connection: ConnectionData,
}

impl Tab {
    fn new(egui_ctx: egui::Context, connection: ConnectionData) -> Self {
        let (tab_send, client_recv) = mpsc::channel::<String>(5);
        let (client_send, tab_recv) = mpsc::channel::<ClientCommands>(100);

        let thread_connection = connection.clone();

        tokio::spawn(async move {
            client::network(client_send, client_recv, egui_ctx, thread_connection).await
        });

        Self {
            messages: Vec::new(),
            send: tab_send,
            recv: tab_recv,
            message: String::new(),
            connect_state: ConnectState::Loading,
            connection,
        }
    }

    fn reconnect(&mut self, egui_ctx: egui::Context) {
        let (tab_send, client_recv) = mpsc::channel::<String>(5);
        let (client_send, tab_recv) = mpsc::channel::<ClientCommands>(100);

        let thread_connection = self.connection.clone();

        tokio::spawn(async move {
            client::network(client_send, client_recv, egui_ctx, thread_connection).await
        });

        self.send = tab_send;
        self.recv = tab_recv;
    }

    fn change_name(&mut self, name: &str) {
        let message = format!("/n {}", name);

        self.send(message);

        self.connection.set_name(name);
    }

    fn send_message(&mut self) {
        let message = self.message.clone();
        self.send(message);
        self.message.clear();
    }

    fn send(&mut self, message: String) {
        let sender = self.send.clone();
        thread::spawn(move || {
            sender.blocking_send(message).unwrap();
        });
    }

    fn sync_messages(&mut self) {
        if self.connect_state != ConnectState::Failed
            || self.connect_state != ConnectState::Disconnect
        {
            loop {
                match self.recv.try_recv() {
                    Ok(ClientCommands::ChatCommand(c)) => self.messages.push(c),
                    Ok(ClientCommands::ConnectState(s)) => self.connect_state = s,

                    Err(TryRecvError::Disconnected) => {
                        self.connect_state = match self.connect_state {
                            ConnectState::Connected => ConnectState::Disconnect,
                            _ => ConnectState::Failed,
                        };

                        break;
                    }

                    Err(TryRecvError::Empty) => break,
                }
            }
        }
    }
}

#[derive(PartialEq, Default)]
enum ServerEdit {
    #[default]
    None,
    New,
    Change(usize),
}

#[derive(Default)]
pub struct Client {
    tabs: Vec<Tab>,

    current_tab: usize,

    server_edit: ServerEdit,
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

        if connections.is_empty() {
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
        // Update data
        for tab in self.tabs.iter_mut() {
            tab.sync_messages();
        }

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                egui::warn_if_debug_build(ui);

                ui.menu_button("Server", |ui| {
                    if ui.button("Edit").clicked() {
                        self.server_edit = ServerEdit::Change(self.current_tab);

                        self.server_edit_address =
                            self.tabs[self.current_tab].connection.server().to_string();
                        self.server_edit_name =
                            self.tabs[self.current_tab].connection.name().to_string();
                    }

                    if ui.button("Reconnect").clicked() {
                        self.tabs[self.current_tab].reconnect(ctx.clone());
                    }

                    if ui
                        .add_enabled(self.tabs.len() != 1, egui::Button::new("Close"))
                        .clicked()
                    {
                        self.tabs.remove(self.current_tab);
                    }
                });
            });
        });

        egui::SidePanel::left("left_panel").show(ctx, |ui| {
            let mut to_remove = Vec::new();
            for (i, tab) in self.tabs.iter().enumerate() {
                ui.horizontal(|ui| {
                    let text = match tab.connect_state {
                        ConnectState::Loading => {
                            ui.spinner();
                            egui::RichText::new(tab.connection.server())
                        }
                        ConnectState::Disconnect | ConnectState::Failed => {
                            egui::RichText::new(tab.connection.server()).color(egui::Color32::RED)
                        }
                        _ => egui::RichText::new(tab.connection.server()),
                    };

                    if ui.button(text).clicked() {
                        self.current_tab = i;
                    }

                    if ui
                        .add_enabled(
                            self.tabs.len() != 1,
                            egui::Button::new(egui::RichText::new("???").color(egui::Color32::RED)),
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
                self.server_edit = ServerEdit::New;
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
                                    for row in 0..self.tabs[self.current_tab].messages.len() {
                                        let c = &self.tabs[self.current_tab].messages[row];

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
                self.tabs[self.current_tab].send_message();
                response.request_focus();
            }
        });

        if self.server_edit != ServerEdit::None {
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
                            match self.server_edit {
                                ServerEdit::New => self.tabs.push(Tab::new(
                                    ctx.clone(),
                                    ConnectionData::new(
                                        &self.server_edit_address,
                                        &self.server_edit_name,
                                    ),
                                )),

                                ServerEdit::Change(i) => {
                                    if self.server_edit_address != *self.tabs[i].connection.server()
                                    {
                                        self.tabs[i] = Tab::new(
                                            ctx.clone(),
                                            ConnectionData::new(
                                                &self.server_edit_address,
                                                &self.server_edit_name,
                                            ),
                                        );
                                    } else {
                                        self.tabs[i].change_name(&self.server_edit_name);
                                    }
                                }

                                _ => unreachable!(),
                            }

                            self.server_edit_address.clear();
                            self.server_edit_name.clear();
                            self.server_edit = ServerEdit::None;
                        }

                        if ui.button("Cancel").clicked() {
                            self.server_edit_address.clear();
                            self.server_edit_name.clear();
                            self.server_edit = ServerEdit::None;
                        }
                    });
                });
        }
    }
}
