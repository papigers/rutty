use std::{borrow::Cow, net::SocketAddr, sync::Arc};

use axum::{
    extract::{
        ws::{close_code, CloseFrame, Message, WebSocket, WebSocketUpgrade},
        ConnectInfo, State,
    },
    response::IntoResponse,
};
use futures_util::{stream::SplitSink, SinkExt, StreamExt};
use tokio::{io::AsyncWriteExt, sync::Notify};
use tracing::{debug, error, info, trace, warn};

use crate::{
    cli::Config,
    command::{self, TerminalSize},
};

pub(crate) async fn handler(
    State(config): State<Config>,
    ws: WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, addr, config))
}

async fn handle_socket(mut socket: WebSocket, address: SocketAddr, config: Config) {
    info!("Connected: {address}");

    loop {
        let message = socket.recv().await;
        match message {
            Some(Ok(message)) => {
                if let CommandMessage::Start(size) = message.into() {
                    debug!("Running command for {address:?}");
                    run_command(socket, address, size, config).await;
                    break;
                }
                continue;
            }
            _ => continue,
        }
    }

    info!("Disconnected: {address}");
}

async fn run_command(
    socket: WebSocket,
    address: SocketAddr,
    terminal_size: Option<TerminalSize>,
    config: Config,
) {
    let (mut sender, receiver) = futures_util::StreamExt::split(socket);

    let mut command = command::Command::new(config.command.as_str());
    command.args(config.args);

    match command.spawn(terminal_size) {
        Err(err) => {
            error!("Failed to spawn command: {err}");
            sender
                .send(Message::Close(Some(CloseFrame {
                    code: close_code::ERROR,
                    reason: Cow::from(err.to_string()),
                })))
                .await
                .unwrap_or_default();
            return;
        }
        _ => (),
    };

    let pid = command
        .pid()
        .map_or("N/A".to_string(), |pid| pid.to_string());
    info!("Command is running for client: {address}. PID: {pid}");

    let aborter = Arc::new(Notify::new());
    let (writer, read_stream) = command.read_and_control(aborter.clone());

    let mut send_task = tokio::spawn(message_sender(read_stream, sender));
    let mut recv_task = tokio::spawn(message_receiver(writer, receiver, config.allow_write));

    let aborter2 = aborter.clone();
    tokio::select! {
        _ = (&mut send_task) => {
            trace!("Command is finished, closing {address}");
            recv_task.abort();
            aborter.notify_waiters();
        },
        _ = (&mut recv_task) => {
            trace!("Socket {address} disconnected");
            send_task.abort();
            aborter2.notify_waiters();
        },
    }
}

async fn message_sender(
    mut stream: command::CommandStream,
    mut sender: SplitSink<WebSocket, Message>,
) {
    while let Some(item) = stream.next().await {
        let message = match item {
            command::CommandStreamItem::Output(bytes) => Some(Message::Binary(bytes.into())),
            command::CommandStreamItem::Error(err) => {
                warn!("Command reported error: {err}");
                None
            }
            command::CommandStreamItem::Exit(reason) => Some(Message::Close(Some(CloseFrame {
                code: close_code::NORMAL,
                reason: Cow::from(reason),
            }))),
        };
        if let Some(message) = message {
            let result = sender.send(message).await;

            if let Some(err) = result.err() {
                warn!("Failed to write to socket: {err}");
                break;
            };
        }
    }
}

#[derive(Debug)]
enum CommandMessage {
    Start(Option<TerminalSize>),
    Input(Vec<u8>),
    Resize(TerminalSize),
    Irrelevant(Message),
}

const COMMAND_MESSAGE_DELIMITER: &str = ";";
const START_PREFIX: char = '0';
const INPUT_PREFIX: char = '1';
const RESIZE_PREFIX: char = '2';

impl From<String> for CommandMessage {
    fn from(value: String) -> Self {
        let split = value.split(COMMAND_MESSAGE_DELIMITER).collect::<Vec<_>>();
        let prefix = match split.get(0) {
            Some(it) if it.len() == 1 => it,
            _ => return CommandMessage::Input(value.as_bytes().into()),
        };
        let prefix = prefix.chars().nth(0).unwrap();

        let fallback = || CommandMessage::Input(value.as_bytes().to_vec());

        let from = match prefix {
            INPUT_PREFIX => CommandMessage::Input((&value[2..]).into()),
            RESIZE_PREFIX | START_PREFIX => {
                let rows = split.get(1).and_then(|n| n.parse::<u16>().ok());
                let cols = split.get(2).and_then(|n| n.parse::<u16>().ok());
                let is_start = prefix == START_PREFIX;
                if rows.is_none() || cols.is_none() {
                    if is_start {
                        warn!("Failed to parse size from start message: {value}");
                        return CommandMessage::Start(None);
                    }

                    warn!("Failed to parse resize message: {value}");
                    return fallback();
                }

                if is_start {
                    return CommandMessage::Start(Some(TerminalSize(rows.unwrap(), cols.unwrap())));
                }

                CommandMessage::Resize(TerminalSize(rows.unwrap(), cols.unwrap()))
            }
            _ => fallback(),
        };

        debug!("Parsed CommandMessage {from:?}");
        from
    }
}

impl From<Message> for CommandMessage {
    fn from(value: Message) -> Self {
        match value {
            Message::Binary(d) => CommandMessage::Input(d),
            Message::Text(t) => t.into(),
            msg => CommandMessage::Irrelevant(msg),
        }
    }
}

async fn message_receiver(
    mut writer: command::CommandWriter,
    mut receiver: futures_util::stream::SplitStream<WebSocket>,
    allow_write: bool,
) {
    while let Some(Ok(msg)) = receiver.next().await {
        debug!("Got msg {msg:?}");
        let result = match CommandMessage::from(msg) {
            CommandMessage::Input(d) => {
                if !allow_write {
                    debug!("Skipping write {d:?}");
                    continue;
                }
                debug!("Writing {d:?}");
                writer.write(d.as_slice()).await
            }
            CommandMessage::Resize(size) => {
                debug!("Resizing to: {size:?}");
                match writer.resize(size.into()) {
                    Err(e) => error!("Failed to resize to {size:?}: {e}"),
                    _ => (),
                };
                Ok(0)
            }
            CommandMessage::Start(_) => {
                warn!("Unexpected start message");
                continue;
            }
            CommandMessage::Irrelevant(inner) => {
                debug!("Got unprocessed message: {inner:?}");
                if let Message::Close(_) = inner {
                    break;
                }
                continue;
            }
        };

        if let Some(err) = result.err() {
            warn!("Failed to write to command: {err}");
            break;
        }
    }
}
