//! WebSocket server for phone sync
//! Handles connections from phone companion app

use super::protocol::{BrowserState, MessagePayload, SyncCommand, SyncEvent, SyncMessage};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

const WS_MAGIC: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

/// Client connection state
#[derive(Debug)]
pub struct Client {
    pub id: u64,
    pub connected_at: Instant,
    pub last_ping: Instant,
    pub subscriptions: Vec<String>,
    stream: TcpStream,
}

/// Sync server manages WebSocket connections
pub struct SyncServer {
    port: u16,
    running: Arc<Mutex<bool>>,
    clients: Arc<Mutex<HashMap<u64, Client>>>,
    next_client_id: Arc<Mutex<u64>>,

    // Channel for commands from phone
    command_tx: Sender<(u64, SyncCommand)>,
    command_rx: Receiver<(u64, SyncCommand)>,

    // Channel for events to phone
    event_tx: Sender<SyncEvent>,
    event_rx: Arc<Mutex<Receiver<SyncEvent>>>,

    // QR code data for easy connection
    qr_data: Option<String>,
}

impl SyncServer {
    pub fn new(port: u16) -> Self {
        let (command_tx, command_rx) = channel();
        let (event_tx, event_rx) = channel();

        Self {
            port,
            running: Arc::new(Mutex::new(false)),
            clients: Arc::new(Mutex::new(HashMap::new())),
            next_client_id: Arc::new(Mutex::new(1)),
            command_tx,
            command_rx,
            event_tx,
            event_rx: Arc::new(Mutex::new(event_rx)),
            qr_data: None,
        }
    }

    /// Start the WebSocket server
    pub fn start(&mut self) -> Result<(), String> {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", self.port))
            .map_err(|e| format!("Failed to bind: {}", e))?;

        listener
            .set_nonblocking(true)
            .map_err(|e| format!("Failed to set nonblocking: {}", e))?;

        *self.running.lock().unwrap() = true;

        // Generate QR data
        if let Ok(ip) = get_local_ip() {
            self.qr_data = Some(format!("sassy://{}:{}", ip, self.port));
        }

        let running = self.running.clone();
        let clients = self.clients.clone();
        let next_id = self.next_client_id.clone();
        let cmd_tx = self.command_tx.clone();
        let event_rx = self.event_rx.clone();

        // Accept connections in background thread
        thread::spawn(move || {
            while *running.lock().unwrap() {
                match listener.accept() {
                    Ok((stream, _addr)) => {
                        let mut id_lock = next_id.lock().unwrap();
                        let client_id = *id_lock;
                        *id_lock += 1;
                        drop(id_lock);

                        let clients_clone = clients.clone();
                        let cmd_tx_clone = cmd_tx.clone();

                        thread::spawn(move || {
                            if let Ok(client) = handle_websocket_handshake(stream, client_id) {
                                clients_clone.lock().unwrap().insert(client_id, client);

                                // Handle client in this thread
                                handle_client(client_id, clients_clone.clone(), cmd_tx_clone);
                            }
                        });
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        // No pending connections
                        thread::sleep(Duration::from_millis(50));
                    }
                    Err(_) => {
                        thread::sleep(Duration::from_millis(100));
                    }
                }
            }
        });

        // Broadcast events in background thread
        let running2 = self.running.clone();
        let clients2 = self.clients.clone();

        thread::spawn(move || {
            while *running2.lock().unwrap() {
                if let Ok(rx) = event_rx.lock() {
                    while let Ok(event) = rx.try_recv() {
                        let msg = SyncMessage::event(event);
                        if let Ok(json) = msg.to_json() {
                            broadcast_to_clients(&clients2, &json);
                        }
                    }
                }
                thread::sleep(Duration::from_millis(10));
            }
        });

        Ok(())
    }

    /// Stop the server
    pub fn stop(&mut self) {
        *self.running.lock().unwrap() = false;
    }

    /// Check if server is running
    pub fn is_running(&self) -> bool {
        *self.running.lock().unwrap()
    }

    /// Get number of connected clients
    pub fn client_count(&self) -> usize {
        self.clients.lock().unwrap().len()
    }

    /// Get QR code data for phone to scan
    pub fn qr_data(&self) -> Option<&str> {
        self.qr_data.as_deref()
    }

    /// Poll for incoming commands (non-blocking)
    pub fn poll_command(&self) -> Option<(u64, SyncCommand)> {
        self.command_rx.try_recv().ok()
    }

    /// Send event to all connected phones
    pub fn broadcast_event(&self, event: SyncEvent) {
        let _ = self.event_tx.send(event);
    }

    /// Send message to specific client
    pub fn send_to_client(&self, client_id: u64, message: SyncMessage) {
        if let Ok(json) = message.to_json() {
            let mut clients = self.clients.lock().unwrap();
            if let Some(client) = clients.get_mut(&client_id) {
                let _ = send_websocket_frame(&mut client.stream, &json);
            }
        }
    }

    /// Send full state to a client
    pub fn send_state(&self, client_id: u64, state: BrowserState) {
        let msg = SyncMessage::state(state);
        self.send_to_client(client_id, msg);
    }

    /// Disconnect a client
    pub fn disconnect_client(&self, client_id: u64) {
        self.clients.lock().unwrap().remove(&client_id);
    }
}

fn get_local_ip() -> Result<String, String> {
    // Try to find local IP
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").map_err(|e| e.to_string())?;
    socket.connect("8.8.8.8:80").map_err(|e| e.to_string())?;
    let addr = socket.local_addr().map_err(|e| e.to_string())?;
    Ok(addr.ip().to_string())
}

fn handle_websocket_handshake(mut stream: TcpStream, client_id: u64) -> Result<Client, String> {
    let mut buffer = [0u8; 4096];
    let n = stream
        .read(&mut buffer)
        .map_err(|e| format!("Read error: {}", e))?;

    let request = String::from_utf8_lossy(&buffer[..n]);

    // Parse WebSocket key
    let key = request
        .lines()
        .find(|line| crate::fontcase::ascii_lower(line).starts_with("sec-websocket-key:"))
        .and_then(|line| line.split(':').nth(1))
        .map(|k| k.trim().to_string())
        .ok_or("Missing Sec-WebSocket-Key")?;

    // Generate accept key
    let accept = {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let combined = format!("{}{}", key, WS_MAGIC);

        // Simple hash (in production use proper SHA-1 + base64)
        let mut hasher = DefaultHasher::new();
        combined.hash(&mut hasher);
        let hash = hasher.finish();
        base64_encode(&hash.to_be_bytes())
    };

    // Send handshake response
    let response = format!(
        "HTTP/1.1 101 Switching Protocols`r`n\
         Upgrade: websocket`r`n\
         Connection: Upgrade`r`n\
         Sec-WebSocket-Accept: {}`r`n`r`n",
        accept
    );

    stream
        .write_all(response.as_bytes())
        .map_err(|e| format!("Write error: {}", e))?;

    stream
        .set_nonblocking(true)
        .map_err(|e| format!("Failed to set nonblocking: {}", e))?;

    let now = Instant::now();
    Ok(Client {
        id: client_id,
        connected_at: now,
        last_ping: now,
        subscriptions: Vec::new(),
        stream,
    })
}

fn handle_client(
    client_id: u64,
    clients: Arc<Mutex<HashMap<u64, Client>>>,
    cmd_tx: Sender<(u64, SyncCommand)>,
) {
    loop {
        let should_continue = {
            let mut clients_lock = clients.lock().unwrap();
            let client = match clients_lock.get_mut(&client_id) {
                Some(c) => c,
                None => return, // Client disconnected
            };

            // Try to read frame
            match read_websocket_frame(&mut client.stream) {
                Ok(Some(data)) => {
                    // Parse message
                    if let Ok(msg) = SyncMessage::from_json(&data) {
                        match msg.payload {
                            MessagePayload::Command(cmd) => {
                                let _ = cmd_tx.send((client_id, cmd));
                            }
                            MessagePayload::Ping => {
                                let pong = SyncMessage::pong();
                                if let Ok(json) = pong.to_json() {
                                    let _ = send_websocket_frame(&mut client.stream, &json);
                                }
                                client.last_ping = Instant::now();
                            }
                            _ => {}
                        }
                    }
                    true
                }
                Ok(None) => true, // No data available
                Err(_) => false,  // Connection closed or error
            }
        };

        if !should_continue {
            clients.lock().unwrap().remove(&client_id);
            return;
        }

        thread::sleep(Duration::from_millis(10));
    }
}

fn read_websocket_frame(stream: &mut TcpStream) -> Result<Option<String>, String> {
    let mut header = [0u8; 2];
    match stream.read_exact(&mut header) {
        Ok(_) => {}
        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => return Ok(None),
        Err(e) => return Err(e.to_string()),
    }

    let _fin = (header[0] & 0x80) != 0;
    let opcode = header[0] & 0x0F;
    let masked = (header[1] & 0x80) != 0;
    let mut length = (header[1] & 0x7F) as usize;

    // Handle close frame
    if opcode == 8 {
        return Err("Connection closed".into());
    }

    // Extended length
    if length == 126 {
        let mut ext = [0u8; 2];
        stream.read_exact(&mut ext).map_err(|e| e.to_string())?;
        length = u16::from_be_bytes(ext) as usize;
    } else if length == 127 {
        let mut ext = [0u8; 8];
        stream.read_exact(&mut ext).map_err(|e| e.to_string())?;
        length = u64::from_be_bytes(ext) as usize;
    }

    // Read mask key if present
    let mask = if masked {
        let mut mask = [0u8; 4];
        stream.read_exact(&mut mask).map_err(|e| e.to_string())?;
        Some(mask)
    } else {
        None
    };

    // Read payload
    let mut payload = vec![0u8; length];
    stream.read_exact(&mut payload).map_err(|e| e.to_string())?;

    // Unmask if needed
    if let Some(mask) = mask {
        for i in 0..payload.len() {
            payload[i] ^= mask[i % 4];
        }
    }

    String::from_utf8(payload)
        .map(Some)
        .map_err(|e| e.to_string())
}

fn send_websocket_frame(stream: &mut TcpStream, data: &str) -> Result<(), String> {
    let payload = data.as_bytes();
    let len = payload.len();

    let mut frame = Vec::new();

    // FIN + text opcode
    frame.push(0x81);

    // Length
    if len < 126 {
        frame.push(len as u8);
    } else if len < 65536 {
        frame.push(126);
        frame.extend_from_slice(&(len as u16).to_be_bytes());
    } else {
        frame.push(127);
        frame.extend_from_slice(&(len as u64).to_be_bytes());
    }

    // Payload (server doesn't mask)
    frame.extend_from_slice(payload);

    stream.write_all(&frame).map_err(|e| e.to_string())
}

fn broadcast_to_clients(clients: &Arc<Mutex<HashMap<u64, Client>>>, message: &str) {
    let mut clients = clients.lock().unwrap();
    let mut disconnected = Vec::new();

    for (id, client) in clients.iter_mut() {
        if send_websocket_frame(&mut client.stream, message).is_err() {
            disconnected.push(*id);
        }
    }

    for id in disconnected {
        clients.remove(&id);
    }
}

fn base64_encode(data: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut result = String::new();
    let chunks = data.chunks(3);

    for chunk in chunks {
        let b0 = chunk[0] as usize;
        let b1 = chunk.get(1).copied().unwrap_or(0) as usize;
        let b2 = chunk.get(2).copied().unwrap_or(0) as usize;

        result.push(ALPHABET[b0 >> 2] as char);
        result.push(ALPHABET[((b0 & 0x03) << 4) | (b1 >> 4)] as char);

        if chunk.len() > 1 {
            result.push(ALPHABET[((b1 & 0x0F) << 2) | (b2 >> 6)] as char);
        } else {
            result.push('=');
        }

        if chunk.len() > 2 {
            result.push(ALPHABET[b2 & 0x3F] as char);
        } else {
            result.push('=');
        }
    }

    result
}
