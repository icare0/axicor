use bevy::prelude::*;
use std::thread;

const GEOM_URL: &str = "127.0.0.1:8001";

#[derive(States, Default, Debug, Clone, Eq, PartialEq, Hash)]
pub enum IdeState {
    #[default]
    Connecting,
    LoadingGeometry,
    Live,
}

#[derive(Resource)]
pub struct GeometryReceiver(pub crossbeam_channel::Receiver<Vec<[f32; 4]>>);

pub struct LoaderPlugin;

impl Plugin for LoaderPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<IdeState>()
           .add_systems(OnEnter(IdeState::Connecting), fetch_real_geometry)
           .add_systems(Update, check_geometry_finished.run_if(in_state(IdeState::LoadingGeometry)));
    }
}

fn fetch_real_geometry(
    mut commands: Commands,
    mut next_state: ResMut<NextState<IdeState>>,
) {
    let (tx, rx) = crossbeam_channel::bounded(1);
    commands.insert_resource(GeometryReceiver(rx));

    thread::spawn(move || {
        use tokio::net::TcpStream;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        
        // We need a tokio runtime for TcpStream
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(async {
            info!("Connecting to GeometryServer at {}...", GEOM_URL);
            
            let mut stream = match TcpStream::connect(GEOM_URL).await {
                Ok(s) => s,
                Err(e) => {
                    error!("GeometryServer connection failed: {}", e);
                    return;
                }
            };

            // Send "GEOM" request magic
            if let Err(e) = stream.write_all(b"GEOM").await {
                error!("Failed to send GEOM request: {}", e);
                return;
            }

            // Read magic and count
            let mut header = [0u8; 8];
            if let Err(e) = stream.read_exact(&mut header).await {
                error!("Failed to read geometry header: {}", e);
                return;
            }

            if &header[0..4] != b"GEOM" {
                error!("Invalid GEOM magic from server");
                return;
            }

            let num_neurons = u32::from_le_bytes(header[4..8].try_into().unwrap()) as usize;
            info!("Server reporting {} neurons", num_neurons);

            let mut buffer = vec![0u8; num_neurons * 16];
            if let Err(e) = stream.read_exact(&mut buffer).await {
                error!("Failed to read geometry data: {}", e);
                return;
            }

            let geometry: Vec<[f32; 4]> = bytemuck::cast_slice(&buffer).to_vec();
            let _ = tx.send(geometry);
        });
    });

    next_state.set(IdeState::LoadingGeometry);
}

fn check_geometry_finished(
    receiver: Res<GeometryReceiver>,
    mut next_state: ResMut<NextState<IdeState>>,
    mut commands: Commands,
) {
    if let Ok(geometry) = receiver.0.try_recv() {
        info!("Geometry received: {} neurons", geometry.len());
        
        // This will be handled in world.rs to initialize the GPU buffers
        // For now, we just store it as a resource
        commands.insert_resource(LoadedGeometry(geometry));
        next_state.set(IdeState::Live);
        commands.remove_resource::<GeometryReceiver>();
    }
}

#[derive(Resource)]
pub struct LoadedGeometry(pub Vec<[f32; 4]>);


