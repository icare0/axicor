use crossbeam_channel::{bounded, Receiver, Sender};
use nokhwa::{Camera, utils::{CameraIndex, RequestedFormat, RequestedFormatType, CameraFormat, FrameFormat, Resolution, KnownCameraControl, ControlValueSetter}};
use nokhwa::pixel_format::LumaFormat;
use std::time::Instant;

/// Плоский буфер кадра. Гарантированно не меняет capacity.
pub struct FrameBuffer {
    pub pixels: Vec<u8>,
    pub _width: u32,
    pub _height: u32,
    pub timestamp: Instant,
}

pub struct CapturePool {
    pub free_tx: Sender<FrameBuffer>,
    pub free_rx: Receiver<FrameBuffer>,
    pub ready_tx: Sender<FrameBuffer>,
    pub ready_rx: Receiver<FrameBuffer>,
}

impl CapturePool {
    /// Аллоцируем буферы один раз при старте
    pub fn new(capacity: usize, frame_bytes: usize, width: u32, height: u32) -> Self {
        let (free_tx, free_rx) = bounded(capacity);
        let (ready_tx, ready_rx) = bounded(capacity);

        for _ in 0..capacity {
            let mut pixels = Vec::with_capacity(frame_bytes);
            unsafe { pixels.set_len(frame_bytes); } // [DOD] Фиксируем длину навсегда
            free_tx.send(FrameBuffer { 
                pixels, _width: width, _height: height, timestamp: Instant::now() 
            }).unwrap();
        }

        Self { free_tx, free_rx, ready_tx, ready_rx }
    }
}

pub fn spawn_capture_thread(pool: CapturePool, _width: u32, _height: u32) {
    std::thread::Builder::new()
        .name("retina-capture".into())
        .spawn(move || {
            let index = CameraIndex::Index(0);
            
            // 1. Форсируем FHD 60 FPS через MJPEG (избегаем bottleneck'а USB)
            let format = CameraFormat::new(Resolution::new(1920, 1080), FrameFormat::MJPEG, 60);
            let requested = RequestedFormat::new::<LumaFormat>(RequestedFormatType::Closest(format));
            
            let mut camera = Camera::new(index, requested).expect("FATAL: Cannot open webcam");
            camera.open_stream().expect("FATAL: Cannot open camera stream");

            println!("📸 [Capture] Camera active: {}", camera.info());
            println!("📸 [Capture] Negotiated format: {:?}", camera.camera_format());

            // 2. РАЗВЕДКА: Читаем, что аппаратно поддерживает твоя камера
            println!("🔍 [Capture] Probing supported controls...");
            let controls = camera.supported_camera_controls().unwrap_or_default();
            for id in controls {
                let control = camera.camera_control(id);
                println!("  - Supported: {:?} | Info: {:?}", id, control);
            }

            // 3. ЖЕСТКАЯ ФИКСАЦИЯ ПАРАМЕТРОВ (V4L2 Raw IDs)
            // Отключаем Dynamic Framerate (гарантируем 30 FPS любой ценой, даже если темно)
            let _ = camera.set_camera_control(KnownCameraControl::Other(10094851), ControlValueSetter::Boolean(false));
            
            // Auto Exposure = Manual (V4L2: 1 = Manual, 3 = Aperture Priority)
            let _ = camera.set_camera_control(KnownCameraControl::Other(10094849), ControlValueSetter::Integer(1));
            
            // Жесткая выдержка (подобрать по свету, обычно 100-300)
            let _ = camera.set_camera_control(KnownCameraControl::Other(10094850), ControlValueSetter::Integer(150));
            
            // Автофокус = Выкл
            let _ = camera.set_camera_control(KnownCameraControl::Other(10094860), ControlValueSetter::Boolean(false));
            
            // Фокус = Фиксированный (бесконечность/комната)
            let _ = camera.set_camera_control(KnownCameraControl::Other(10094858), ControlValueSetter::Integer(20));

            loop {
                let mut frame = match pool.free_rx.recv() {
                    Ok(f) => f,
                    Err(_) => break,
                };

                let new_frame = camera.frame().expect("FATAL: Camera drop");
                let decoded = new_frame.decode_image::<LumaFormat>().unwrap();
                let bytes = decoded.as_raw();
                
                let len = bytes.len().min(frame.pixels.len());
                unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), frame.pixels.as_mut_ptr(), len); }
                frame.timestamp = std::time::Instant::now();

                if pool.ready_tx.send(frame).is_err() { break; }
            }
        }).unwrap();
}
