use anyhow::{Context, Result};
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, oneshot};

use super::slow_path::{GeometryRequest, GeometryResponse};

/// Connects to a GeometryServer, sends a request, and waits for a response.
pub async fn send_geometry_request(
    target_addr: SocketAddr,
    req: &GeometryRequest,
) -> Result<GeometryResponse> {
    let mut stream = TcpStream::connect(target_addr).await
        .with_context(|| format!("Failed to connect to GeometryServer at {}", target_addr))?;

    // Serialize payload
    let encoded = bincode::serialize(req).context("Failed to serialize GeometryRequest")?;

    // Send Length-prefix (4 bytes, little-endian)
    let len = encoded.len() as u32;
    stream.write_all(&len.to_le_bytes()).await?;

    // Send payload
    stream.write_all(&encoded).await?;

    // Read Length-prefix of response
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).await
        .context("Failed to read length prefix from GeometryResponse")?;
    let resp_len = u32::from_le_bytes(len_buf) as usize;

    // Read payload
    let mut resp_buf = vec![0u8; resp_len];
    stream.read_exact(&mut resp_buf).await
        .context("Failed to read payload from GeometryResponse")?;

    let resp: GeometryResponse = bincode::deserialize(&resp_buf)
        .context("Failed to deserialize GeometryResponse")?;

    Ok(resp)
}

/// A TCP Server that accepts structural graph updates (GeometryRequest)
/// from neighboring shards and passes them via MPSC to the Orchestrator.
pub struct GeometryServer {
    listener: TcpListener,
}

impl GeometryServer {
    /// Binds the server to the provided SocketAddr.
    pub async fn bind(addr: SocketAddr) -> Result<Self> {
        let listener = TcpListener::bind(addr).await?;
        Ok(Self { listener })
    }

    /// Returns the active local socket address
    pub fn local_addr(&self) -> Result<SocketAddr> {
        Ok(self.listener.local_addr()?)
    }

    /// Spawns the server loop in a Tokio task and returns a receiver for incoming geometry requests.
    /// The orchestrator drains this receiver during the Night Phase.
    pub fn spawn(self) -> mpsc::Receiver<(GeometryRequest, oneshot::Sender<GeometryResponse>)> {
        // We use a reasonably small buffer since geometry requests are only processed during the Night Phase.
        let (tx, rx) = mpsc::channel(100);
        
        tokio::spawn(async move {
            loop {
                let (mut stream, _) = match self.listener.accept().await {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                
                let tx_clone = tx.clone();
                tokio::spawn(async move {
                    let mut len_buf = [0u8; 4];
                    if stream.read_exact(&mut len_buf).await.is_err() { return; }
                    let req_len = u32::from_le_bytes(len_buf) as usize;
                    
                    // Simple protection against insanely large payloads
                    if req_len > 1024 * 1024 { return; }
                    
                    let mut req_buf = vec![0u8; req_len];
                    if stream.read_exact(&mut req_buf).await.is_err() { return; }
                    
                    let req: GeometryRequest = match bincode::deserialize(&req_buf) {
                        Ok(r) => r,
                        Err(_) => return,
                    };
                    
                    let (resp_tx, resp_rx) = oneshot::channel();
                    
                    // Route to the Orchestrator
                    if tx_clone.send((req, resp_tx)).await.is_err() {
                        return; // Receiver dropped, server shutting down
                    }
                    
                    // Wait for Orchestrator to fulfill request
                    let resp = match resp_rx.await {
                        Ok(r) => r,
                        Err(_) => return, // Orchestrator dropped the response channel
                    };
                    
                    // Return result to sender
                    if let Ok(encoded_resp) = bincode::serialize(&resp) {
                        let resp_len = encoded_resp.len() as u32;
                        let _ = stream.write_all(&resp_len.to_le_bytes()).await;
                        let _ = stream.write_all(&encoded_resp).await;
                    }
                });
            }
        });
        
        rx
    }
}
