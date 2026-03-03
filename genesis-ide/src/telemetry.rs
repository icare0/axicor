use bevy::{
    prelude::*,
    render::{
        render_resource::{StorageBuffer, ShaderType},
        renderer::{RenderDevice, RenderQueue},
    },
};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};
use std::thread;
use tokio::runtime::Builder;
use futures_util::StreamExt;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use std::convert::TryInto;

const WS_URL: &str = "ws://127.0.0.1:8002/ws";

#[derive(Event, Clone)]
pub struct SpikeFrameEvent {
    pub tick: u64,
    pub spike_ids: Vec<u32>,
}

#[derive(Resource)]
pub struct TelemetryReceiver(pub UnboundedReceiver<(u64, Vec<u32>)>);

#[derive(ShaderType, Default, Clone)]
pub struct SpikeData {
    #[size(runtime)]
    pub intensities: Vec<f32>,
}

#[derive(Resource, Default)]
pub struct GpuSpikeBuffer {
    pub buffer: StorageBuffer<SpikeData>,
    pub initialized: bool,
}

pub struct TelemetryPlugin;

impl Plugin for TelemetryPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SpikeFrameEvent>()
            .init_resource::<GpuSpikeBuffer>()
            .add_systems(Startup, setup_telemetry_socket)
            .add_systems(Update, (drain_telemetry_socket, apply_telemetry_spikes).chain());
    }
}

pub fn setup_telemetry_socket(mut commands: Commands) {
    let (tx, rx) = unbounded_channel();

    // Создаем выделенный поток ОС для сетевого реактора
    thread::spawn(move || {
        // Поднимаем изолированный Tokio Runtime
        let rt = Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("FATAL: Failed to build isolated Tokio runtime for Telemetry");

        rt.block_on(async move {
            info!("Connecting to Genesis Telemetry at {}...", WS_URL);
            let mut ws_stream = match connect_async(WS_URL).await {
                Ok((stream, _)) => stream,
                Err(e) => {
                    error!("Telemetry WS connection failed: {}", e);
                    return;
                }
            };

            info!("Telemetry connected. Awaiting frames...");
            while let Some(msg) = ws_stream.next().await {
                if let Ok(Message::Binary(data)) = msg {
                    if let Some((tick, spike_ids)) = decode_telemetry_frame(&data) {
                        let _ = tx.send((tick, spike_ids));
                    }
                }
            }
        });
    });

    commands.insert_resource(TelemetryReceiver(rx));
}

fn decode_telemetry_frame(data: &[u8]) -> Option<(u64, Vec<u32>)> {
    if data.len() < 16 { return None; }
    
    // SPIK magic check
    if &data[0..4] != b"SPIK" { return None; }
    
    let tick = u64::from_le_bytes(data[4..12].try_into().unwrap());
    let count = u32::from_le_bytes(data[12..16].try_into().unwrap()) as usize;
    
    if data.len() < 16 + count * 4 { return None; }
    
    let spikes = data[16..]
        .chunks_exact(4)
        .take(count)
        .map(|c| u32::from_le_bytes(c.try_into().unwrap()))
        .collect();
        
    Some((tick, spikes))
}

pub fn drain_telemetry_socket(
    mut receiver: ResMut<TelemetryReceiver>,
    mut ev_writer: EventWriter<SpikeFrameEvent>,
) {
    while let Ok((tick, spike_ids)) = receiver.0.try_recv() {
        ev_writer.send(SpikeFrameEvent { tick, spike_ids });
    }
}

pub fn apply_telemetry_spikes(
    mut events: EventReader<SpikeFrameEvent>,
    mut gpu_buffer: ResMut<GpuSpikeBuffer>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
    if !gpu_buffer.initialized {
        // Initialize with default capacity (e.g. 500k neurons) if not done
        // This normally happens when the connectome is loaded.
        return;
    }

    let data = gpu_buffer.buffer.get_mut();
    let mut is_dirty = false;

    // 1. Быстрый Decay (линейный проход L1 Cache)
    for glow in data.intensities.iter_mut() {
        if *glow > 0.0 {
            *glow = (*glow - 0.15).max(0.0);
            is_dirty = true;
        }
    }

    // 2. Инъекция (O(K))
    for frame in events.read() {
        for &id in &frame.spike_ids {
            if let Some(glow) = data.intensities.get_mut(id as usize) {
                *glow = 1.0;
                is_dirty = true;
            }
        }
    }

    // 3. Асинхронная заливка в VRAM
    if is_dirty {
        gpu_buffer.buffer.write_buffer(&render_device, &render_queue);
    }
}
