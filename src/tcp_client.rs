use encryption::Cipher;
use once_cell::sync::Lazy;
use std::net::ToSocketAddrs;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::{broadcast, Mutex};
use tokio::time::timeout;

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

async fn establish_connection(addr: &str, key: &str) -> Result<(), String> {
    let cipher = Cipher::new(key, 30);

    let socket_addrs: Vec<_> = addr
        .to_socket_addrs()
        .map_err(|e| format!("Failed to resolve address '{}': {}", addr, e))?
        .collect();

    if socket_addrs.is_empty() {
        return Err(format!("No valid addresses found for '{}'", addr));
    }

    let mut stream = timeout(Duration::from_secs(5), TcpStream::connect(&socket_addrs[0]))
        .await
        .map_err(|_| "Connection timeout".to_string())?
        .map_err(|e| format!("Connection failed to '{}': {}", addr, e))?;

    let local_ip = stream
        .local_addr()
        .map_err(|e| format!("Failed to get local address: {}", e))?
        .ip()
        .to_string();

    let mut auth_message = cipher.encrypt_message(&local_ip);
    auth_message.push('\n');

    stream
        .write_all(auth_message.as_bytes())
        .await
        .map_err(|e| format!("Failed to send authentication: {}", e))?;

    let mut buffer = vec![0u8; 256];
    let n = timeout(Duration::from_secs(5), stream.read(&mut buffer))
        .await
        .map_err(|_| "Authentication timeout".to_string())?
        .map_err(|e| format!("Failed to read authentication response: {}", e))?;

    let response = String::from_utf8_lossy(&buffer[..n]).to_string();

    if !response.contains("authentication successful") {
        return Err(format!("Authentication failed: {}", response));
    }

    *TCP_CONNECTION.lock().await = Some(TcpConnection { stream, cipher });

    Ok(())
}

pub async fn connect(addr: &str, key: &str) -> Result<(), String> {
    establish_connection(addr, key).await?;

    let addr_owned = addr.to_string();
    let key_owned = key.to_string();

    tokio::spawn(async move {
        loop {
            let mut buffer = vec![0u8; 256];
            let mut consecutive_errors = 0;

            loop {
                let has_connection = TCP_CONNECTION.lock().await.is_some();

                if !has_connection {
                    break;
                }

                let read_result = {
                    let mut guard = TCP_CONNECTION.lock().await;
                    let connection = match guard.as_mut() {
                        Some(conn) => conn,
                        None => break,
                    };

                    timeout(Duration::from_secs(2), connection.stream.read(&mut buffer)).await
                };

                match read_result {
                    Ok(Ok(n)) if n > 0 => {
                        let message = String::from_utf8_lossy(&buffer[..n]).to_string();
                        let _ = MESSAGE_BROADCAST.send(message);
                        consecutive_errors = 0;
                    }
                    Ok(Ok(_)) => {
                        eprintln!("[TCP] Connection closed by server");
                        break;
                    }
                    Ok(Err(e)) => {
                        consecutive_errors += 1;
                        eprintln!("[TCP] Read error ({}): {}", consecutive_errors, e);
                        if consecutive_errors > 3 {
                            break;
                        }
                        tokio::time::sleep(Duration::from_millis(500)).await;
                    }
                    Err(_) => continue,
                }
            }

            *TCP_CONNECTION.lock().await = None;
            eprintln!("[TCP] Attempting reconnection in 5 seconds...");
            tokio::time::sleep(Duration::from_secs(5)).await;

            match establish_connection(&addr_owned, &key_owned).await {
                Ok(_) => println!("[TCP] Reconnected successfully"),
                Err(e) => eprintln!("[TCP] Reconnection failed: {}", e),
            }
        }
    });

    Ok(())
}

pub async fn send_command(cmd: &str) -> Result<String, String> {
    let mut guard = TCP_CONNECTION.lock().await;
    let connection = guard.as_mut().ok_or("Not connected")?;

    let mut encrypted = connection.cipher.encrypt_message(cmd);
    encrypted.push_str("\r\n");

    timeout(
        Duration::from_secs(3),
        connection.stream.write_all(encrypted.as_bytes()),
    )
        .await
        .map_err(|_| "Write timeout".to_string())?
        .map_err(|e| format!("Write failed: {}", e))?;

    let mut buffer = vec![0u8; 256];
    match timeout(Duration::from_secs(3), connection.stream.read(&mut buffer)).await {
        Ok(Ok(n)) if n > 0 => Ok(String::from_utf8_lossy(&buffer[..n]).to_string()),
        Ok(Ok(_)) => Ok(String::new()),
        Ok(Err(e)) => Err(format!("Read failed: {}", e)),
        Err(_) => Ok(String::new()),
    }
}