use anyhow::Result;
use common::dc09::DC09Message;
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tokio_tungstenite::tungstenite::Message;

/// Maximum number of messages to buffer in the broadcast channel.
/// This controls backpressure - if clients can't keep up, older messages may be dropped.
const BROADCAST_CHANNEL_CAPACITY: usize = 100;

/// Represents an alarm message in JSON format for websocket transmission.
#[derive(Debug, Clone, Serialize)]
pub struct AlarmJson {
    /// ID token of the message (e.g., "NULL", "SIA-DCS", "ADM-CID")
    pub token: String,
    /// Message sequence number
    pub sequence: u16,
    /// Optional receiver identifier (e.g., "R001")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receiver: Option<String>,
    /// Optional line prefix identifier (e.g., "L001")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_prefix: Option<String>,
    /// Account number
    pub account: String,
    /// Optional message data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    /// Extended data fields
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub extended: Vec<String>,
    /// Optional timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
}

impl From<&DC09Message> for AlarmJson {
    fn from(msg: &DC09Message) -> Self {
        Self {
            token: msg.token.clone(),
            sequence: msg.sequence,
            receiver: msg.receiver.clone(),
            line_prefix: msg.line_prefix.clone(),
            account: msg.account.clone(),
            data: msg.data.clone(),
            extended: msg.extended.clone(),
            timestamp: msg.timestamp.clone(),
        }
    }
}

/// Websocket server that broadcasts alarm messages to connected clients.
pub struct WebSocketServer {
    listener: TcpListener,
    broadcast_tx: broadcast::Sender<String>,
}

impl WebSocketServer {
    /// Creates a new websocket server listening on the specified address and port.
    pub async fn new(address: &str, port: u16) -> Result<Self> {
        let listener = TcpListener::bind(format!("{}:{}", address, port)).await?;
        let (broadcast_tx, _) = broadcast::channel(BROADCAST_CHANNEL_CAPACITY);
        
        log::info!("WebSocket server listening on {}:{}", address, port);
        
        Ok(Self {
            listener,
            broadcast_tx,
        })
    }

    /// Returns a sender that can be used to broadcast messages to all connected clients.
    pub fn get_broadcaster(&self) -> broadcast::Sender<String> {
        self.broadcast_tx.clone()
    }

    /// Runs the websocket server, accepting connections and handling clients.
    pub async fn run(&mut self) -> Result<()> {
        loop {
            match self.listener.accept().await {
                Ok((stream, addr)) => {
                    let rx = self.broadcast_tx.subscribe();
                    tokio::spawn(handle_client(stream, addr, rx));
                }
                Err(e) => log::error!("error accepting websocket connection: {e}"),
            }
        }
    }
}

/// Broadcasts an alarm message to all connected websocket clients.
pub fn broadcast_alarm(broadcaster: &broadcast::Sender<String>, msg: &DC09Message) {
    let alarm_json = AlarmJson::from(msg);
    
    if let Ok(json_str) = serde_json::to_string(&alarm_json) {
        // Broadcast to all connected clients
        // We don't care if there are no receivers
        let _ = broadcaster.send(json_str);
    } else {
        log::error!("failed to serialize alarm to JSON");
    }
}

async fn handle_client(
    stream: TcpStream,
    addr: SocketAddr,
    mut rx: broadcast::Receiver<String>,
) {
    log::debug!("websocket client connected from {}", addr);

    let ws_stream = match tokio_tungstenite::accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            log::error!("error during websocket handshake: {e}");
            return;
        }
    };

    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    // Spawn a task to forward broadcast messages to this client
    let send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if ws_sender
                .send(Message::Text(msg.into()))
                .await
                .is_err()
            {
                break;
            }
        }
    });

    // Handle incoming messages from the client (just drain them, we don't process client messages)
    let recv_task = tokio::spawn(async move {
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(Message::Close(_)) => break,
                Ok(_) => {} // Ignore other message types
                Err(_) => break,
            }
        }
    });

    // Wait for either task to complete
    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
    }

    log::debug!("websocket client disconnected from {}", addr);
}
