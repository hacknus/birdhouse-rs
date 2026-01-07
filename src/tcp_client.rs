use encryption::Cipher;
use once_cell::sync::Lazy;
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::Mutex;
use std::time::Duration;
use tokio::sync::broadcast;

static MESSAGE_BROADCAST: Lazy<broadcast::Sender<String>> = Lazy::new(|| {
    let (tx, _) = broadcast::channel(100);
    tx
});

pub fn subscribe_to_tcp_messages() -> broadcast::Receiver<String> {
    MESSAGE_BROADCAST.subscribe()
}
struct TcpConnection {
    stream: TcpStream,
    cipher: Cipher,
}

static TCP_CONNECTION: Lazy<Mutex<Option<TcpConnection>>> = Lazy::new(|| Mutex::new(None));

pub fn connect(addr: &str, key: &str) -> Result<(), String> {
    let cipher = Cipher::new(key, 30);

    // Resolve the address
    let socket_addrs: Vec<_> = addr
        .to_socket_addrs()
        .map_err(|e| format!("Failed to resolve address '{}': {}", addr, e))?
        .collect();

    if socket_addrs.is_empty() {
        return Err(format!("No valid addresses found for '{}'", addr));
    }

    let mut stream = TcpStream::connect_timeout(&socket_addrs[0], Duration::from_secs(5))
        .map_err(|e| format!("Connection failed to '{}': {}", addr, e))?;

    // Authenticate: send encrypted local IP address with \n terminator
    let local_ip = stream
        .local_addr()
        .map_err(|e| format!("Failed to get local address: {}", e))?
        .ip()
        .to_string();

    println!("key: '{:?}'", key);
    println!("Local IP: '{}'", local_ip);
    println!("Socket Addrs: '{:?}'", socket_addrs);

    let mut auth_message = cipher.encrypt_message(&local_ip);
    auth_message.push('\n'); // Use \n for authentication (matches original tcp.rs)

    println!("Auth msg: '{:?}'", auth_message);

    stream
        .write_all(auth_message.as_bytes())
        .map_err(|e| format!("Failed to send authentication: {}", e))?;

    // Wait for authentication response
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|e| format!("Failed to set timeout: {}", e))?;

    let mut buffer = vec![0u8; 256];
    let n = stream
        .read(&mut buffer)
        .map_err(|e| format!("Failed to read authentication response: {}", e))?;

    let response = String::from_utf8_lossy(&buffer[..n]).to_string();

    if !response.contains("authentication successful") {
        return Err(format!("Authentication failed: {}", response));
    }

    // Set shorter read timeout for normal operation
    stream
        .set_read_timeout(Some(Duration::from_millis(500)))
        .map_err(|e| format!("Failed to set timeout: {}", e))?;

    let mut reader_stream = stream
        .try_clone()
        .map_err(|e| format!("Failed to clone stream: {}", e))?;

    *TCP_CONNECTION.lock().unwrap() = Some(TcpConnection { stream, cipher });

    // Spawn background task to read TCP messages
    std::thread::spawn(move || {
        let mut buffer = vec![0u8; 256];
        loop {
            match reader_stream.read(&mut buffer) {
                Ok(n) if n > 0 => {
                    // Trim null bytes and convert to string
                    let message = String::from_utf8_lossy(&buffer[..n])
                        .trim_matches('\0')
                        .trim()
                        .to_string();

                    if !message.is_empty() {
                        println!("[SERVER] Received: {}", message);
                        let _ = MESSAGE_BROADCAST.send(message);
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(10));
                    continue;
                }
                _ => break,
            }
        }
    });

    Ok(())
}

pub fn send_command(cmd: &str) -> Result<String, String> {
    let mut guard = TCP_CONNECTION.lock().unwrap();
    let connection = guard.as_mut().ok_or("Not connected")?;

    // Encrypt the command and add \r\n terminator (matches original tcp.rs)
    let mut encrypted = connection.cipher.encrypt_message(cmd);
    encrypted.push_str("\r\n");

    connection
        .stream
        .write_all(encrypted.as_bytes())
        .map_err(|e| format!("Write failed: {}", e))?;

    // Read response
    let mut buffer = vec![0u8; 256];
    match connection.stream.read(&mut buffer) {
        Ok(n) if n > 0 => {
            let response = String::from_utf8_lossy(&buffer[..n]).to_string();
            Ok(response)
        }
        Ok(_) => Ok(String::new()),
        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(String::new()),
        Err(e) => Err(format!("Read failed: {}", e)),
    }
}
