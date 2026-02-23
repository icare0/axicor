use bevy::prelude::*;
use std::sync::{mpsc, Arc, Mutex};
use tokio::runtime::Runtime as TokioRuntime;

use crate::{world::SpikeFrame, AppState};

pub struct TelemetryPlugin;

impl Plugin for TelemetryPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Running), start_telemetry_task)
            .add_systems(
                Update,
                drain_telemetry.run_if(in_state(AppState::Running)),
            );
    }
}

/// Cross-thread channel to receive spike frames from the WS task.
/// Wrapped in Mutex so it's Sync (required by Bevy Resource).
#[derive(Resource)]
pub struct TelemetryChannel {
    pub rx: Arc<Mutex<mpsc::Receiver<SpikeFrame>>>,
}

/// Separate sender resource so systems can clone the tx without Res field access issues.
#[derive(Resource, Clone)]
pub struct TelemetrySender(pub mpsc::SyncSender<SpikeFrame>);

/// TelemetryFrameHeader layout (matches genesis-runtime/src/network/telemetry.rs):
/// magic: u32 (GNSS), tick: u64, spikes_count: u32 → 16 bytes total, #[repr(C)] LE.
const HEADER_SIZE: usize = 16;
const MAGIC_GNSS: u32 = u32::from_le_bytes(*b"GNSS");

pub fn setup_telemetry_resources(app: &mut App) {
    let (tx, rx) = mpsc::sync_channel::<SpikeFrame>(64);
    app.insert_resource(TelemetryChannel {
        rx: Arc::new(Mutex::new(rx)),
    });
    app.insert_resource(TelemetrySender(tx));
}

fn start_telemetry_task(sender: Res<TelemetrySender>) {
    let tx = sender.0.clone();

    // Spawn a dedicated OS thread with its own tokio runtime so we don't
    // block Bevy's world while waiting on WS frames.
    std::thread::spawn(move || {
        let rt = TokioRuntime::new().expect("tokio runtime");
        rt.block_on(async move {
            ws_loop(tx).await;
        });
    });
}

const WS_URL: &str = "ws://127.0.0.1:8002/ws";

async fn ws_loop(tx: mpsc::SyncSender<SpikeFrame>) {
    use futures_util::StreamExt;
    use tokio_tungstenite::{connect_async, tungstenite::Message};

    loop {
        println!("[telemetry] Connecting to {}...", WS_URL);
        match connect_async(WS_URL).await {
            Ok((mut ws, _)) => {
                println!("[telemetry] Connected.");
                loop {
                    match ws.next().await {
                        Some(Ok(Message::Binary(data))) => {
                            if let Some(frame) = parse_frame(&data) {
                                // If receiver is full, drop oldest — never block.
                                let _ = tx.try_send(frame);
                            }
                        }
                        Some(Ok(_)) => {} // text/ping — ignore
                        Some(Err(e)) => {
                            eprintln!("[telemetry] WS error: {e}");
                            break;
                        }
                        None => {
                            println!("[telemetry] WS closed.");
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("[telemetry] Connection failed: {e}. Retrying in 2s...");
            }
        }
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }
}

/// Parse binary frame from TelemetryFrameHeader + u32[] payload.
fn parse_frame(data: &[u8]) -> Option<SpikeFrame> {
    if data.len() < HEADER_SIZE {
        return None;
    }

    let magic = u32::from_le_bytes(data[0..4].try_into().ok()?);
    if magic != MAGIC_GNSS {
        eprintln!("[telemetry] Bad magic: {:#010x}", magic);
        return None;
    }

    let tick = u64::from_le_bytes(data[4..12].try_into().ok()?);
    let spikes_count = u32::from_le_bytes(data[12..16].try_into().ok()?) as usize;

    let payload = &data[HEADER_SIZE..];
    if payload.len() < spikes_count * 4 {
        return None;
    }

    let spike_ids: Vec<u32> = payload[..spikes_count * 4]
        .chunks_exact(4)
        .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
        .collect();

    Some(SpikeFrame { tick, spike_ids })
}

/// Drain the channel and emit Bevy events each frame.
fn drain_telemetry(
    channel: Res<TelemetryChannel>,
    mut ev_writer: EventWriter<SpikeFrame>,
) {
    let Ok(rx) = channel.rx.try_lock() else { return };
    while let Ok(frame) = rx.try_recv() {
        ev_writer.send(frame);
    }
}
