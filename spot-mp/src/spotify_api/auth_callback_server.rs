use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::{Duration, Instant};
use std::thread;
use log::{info};
use crate::domain::spot_mp_errors::SpotMpError;

pub struct AuthCallbackServer;

impl AuthCallbackServer {
    /// Starts a temporary server, waits for the Spotify callback (with a 2-minute timeout), and returns the authorization code.
    pub fn listen_for_code() -> Result<String, SpotMpError> {
        // Set up the callback direction
        let listener = TcpListener::bind("127.0.0.1:8888")?;

        // 1. Set the listener to non-blocking mode so it doesn't freeze the thread forever
        listener.set_nonblocking(true)?;
        
        info!("Waiting for authorization in the web browser (Timeout: 2 minutes)...");

        // 2. Define our timeout limit and start the stopwatch
        let timeout_duration = Duration::from_secs(120); 
        let start_time = Instant::now();

        // Use a continuous loop instead of `for stream in listener.incoming()`
        loop {
            // 3. Check if we exceeded the time limit
            if start_time.elapsed() > timeout_duration {
                return Err(SpotMpError::TimeoutTCP());
            }

            // 4. Try to accept an incoming connection
            match listener.accept() {
                Ok((mut stream, _addr)) => {
                    // Set a read timeout on the stream just in case the browser connects but sends nothing
                    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
                    
                    let mut buffer = [0; 2048];
                    // Read the raw HTTP request sent by the browser
                    if stream.read(&mut buffer).is_err() {
                        continue; // If reading fails, ignore and keep listening
                    }
                    
                    let request = String::from_utf8_lossy(&buffer);
                    
                    // Parse the first line of the request
                    if let Some(line) = request.lines().next() {
                        
                        // Check if the user accepted and Spotify sent the code
                        if line.starts_with("GET") && line.contains("code=") {
                            
                            let code_start = line.find("code=").unwrap() + 5;
                            let code_end = line[code_start..].find(&[' ', '&'][..]).unwrap_or(line[code_start..].len());
                            let code = line[code_start..code_start + code_end].to_string();

                            let response = "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=UTF-8\r\n\r\n\
                                <html>\
                                <body style=\"font-family: Arial, sans-serif; text-align: center; margin-top: 50px;\">\
                                    <h2 style=\"color: #1DB954;\">Spot-MP Authorized Successfully!</h2>\
                                    <p>You can now close this window and return to your terminal.</p>\
                                </body>\
                                </html>";
                            
                            stream.write_all(response.as_bytes()).unwrap();
                            stream.flush().unwrap();

                            return Ok(code);
                        } 
                        // Check if the user clicked "Cancel" on the Spotify authorization page
                        else if line.contains("error=") {
                            let response = "HTTP/1.1 400 Bad Request\r\nContent-Type: text/html; charset=UTF-8\r\n\r\n\
                                <html>\
                                <body style=\"font-family: Arial, sans-serif; text-align: center; margin-top: 50px;\">\
                                    <h2 style=\"color: #E22134;\">Authorization Denied</h2>\
                                    <p>Permissions were not granted. Please return to the terminal.</p>\
                                </body>\
                                </html>";
                            
                            stream.write_all(response.as_bytes()).unwrap();
                            stream.flush().unwrap();
                            
                            return Err(SpotMpError::AuthDenied());
                        }
                    }
                },
                // 5. If there is no connection yet, it throws a "WouldBlock" error.
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // Sleep the thread for 100 milliseconds to avoid 100% CPU usage while waiting
                    thread::sleep(Duration::from_millis(100));
                },
                // 6. Handle any other fatal server errors
                Err(e) => {
                    return Err(e.into());
                }
            }
        }
    }
}
