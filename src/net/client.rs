use crate::net::{commands::*, connection::ConnectionData};

use std::sync::{Arc, Mutex};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
    sync::mpsc,
};

pub async fn network(
    commands: Arc<Mutex<Vec<ChatCommands>>>,
    mut recv: mpsc::Receiver<String>,
    egui_ctx: egui::Context,
    connection: ConnectionData,
) {
    // Connect to server
    let server_name = if connection.server().contains(':') {
        connection.server().to_owned()
    } else {
        connection.server().to_owned() + ":6078"
    };

    let stream = TcpStream::connect(server_name).await.unwrap();
    let (reader, mut writer) = stream.into_split();
    let reader = BufReader::new(reader);
    let name = connection.name().clone();

    // Start thread handling user input
    tokio::spawn(async move {
        writer.write_all(name.as_bytes()).await.unwrap();
        writer.write_u8(b'\n').await.unwrap();

        while let Some(command) = recv.recv().await {
            let mut chars = command.chars();
            let c = if chars.next() == Some('/') {
                chars.collect::<String>() + "\n"
            } else {
                format!("m {}\n", command)
            };

            writer.write_all(c.as_bytes()).await.unwrap();
            writer.flush().await.unwrap();
        }
    });

    // Handle TcpStream
    let mut lines = reader.lines();

    while let Some(line) = lines.next_line().await.unwrap() {
        let mut lock = commands.lock().unwrap();
        let command = str::parse::<ChatCommands>(&line).unwrap();
        lock.push(command);
        drop(lock);

        egui_ctx.request_repaint();
    }
}
