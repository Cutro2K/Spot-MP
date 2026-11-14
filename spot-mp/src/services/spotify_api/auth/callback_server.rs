use crate::domain::spot_mp_errors::SpotMpError;
use log::info;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::time::{Instant, timeout};

pub struct AuthCallbackServer;

impl AuthCallbackServer {
    /// Starts a temporary async TCP server, waits for the Spotify callback (with a 2-minute timeout),
    /// and returns the authorization code without blocking the Tokio runtime.
    pub async fn listen_for_code(addr: &str) -> Result<String, SpotMpError> {
        let listener = TcpListener::bind(addr).await?;

        info!("Waiting for authorization in the web browser (Timeout: 2 minutes)...");

        let timeout_duration = Duration::from_secs(120);
        let deadline = Instant::now() + timeout_duration;

        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(SpotMpError::TimeoutTCP());
            }

            match timeout(remaining, listener.accept()).await {
                Ok(Ok((mut stream, _addr))) => {
                    let mut buffer = [0u8; 2048];
                    let bytes_read = match timeout(Duration::from_secs(5), stream.read(&mut buffer)).await {
                        Ok(Ok(n)) if n > 0 => n,
                        _ => continue,
                    };

                    let request = String::from_utf8_lossy(&buffer[..bytes_read]);

                    if let Some(line) = request.lines().next() {
                        if line.starts_with("GET") && line.contains("code=") {
                            let code_start = line.find("code=").unwrap() + 5;
                            let code_end = line[code_start..]
                                .find(&[' ', '&'][..])
                                .unwrap_or(line[code_start..].len());
                            let code = line[code_start..code_start + code_end].to_string();

                            let response = "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=UTF-8\r\n\r\n\
                                <html>\
                                <body style=\"font-family: Arial, sans-serif; text-align: center; margin-top: 50px;\">\
                                    <h2 style=\"color: #1DB954;\">Spot-MP Authorized Successfully!</h2>\
                                    <p>You can now close this window and return to your terminal.</p>\
                                </body>\
                                </html>";

                            let _ = stream.write_all(response.as_bytes()).await;
                            let _ = stream.flush().await;

                            return Ok(code);
                        } else if line.contains("error=") {
                            let response = "HTTP/1.1 400 Bad Request\r\nContent-Type: text/html; charset=UTF-8\r\n\r\n\
                                <html>\
                                <body style=\"font-family: Arial, sans-serif; text-align: center; margin-top: 50px;\">\
                                    <h2 style=\"color: #E22134;\">Authorization Denied</h2>\
                                    <p>Permissions were not granted. Please return to the terminal.</p>\
                                </body>\
                                </html>";

                            let _ = stream.write_all(response.as_bytes()).await;
                            let _ = stream.flush().await;

                            return Err(SpotMpError::AuthDenied());
                        }
                    }
                }
                Ok(Err(e)) => {
                    return Err(e.into());
                }
                Err(_) => {
                    return Err(SpotMpError::TimeoutTCP());
                }
            }
        }
    }
}
