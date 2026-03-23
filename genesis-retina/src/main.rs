mod capture;

use capture::{CapturePool, spawn_capture_thread};
use std::time::Instant;

fn main() {
    // Стандартное разрешение вебки
    let width = 640;
    let height = 480;
    let bytes_per_pixel = 1;
    let frame_bytes = (width * height * bytes_per_pixel) as usize;

    // Пул на 3 кадра: 1 пишется, 1 считается, 1 в запасе.
    let pool = CapturePool::new(3, frame_bytes, width, height);

    // Клонируем сендеры/ресиверы для передачи в потоки
    let capture_pool = CapturePool {
        free_tx: pool.free_tx.clone(),
        free_rx: pool.free_rx.clone(),
        ready_tx: pool.ready_tx.clone(),
        ready_rx: pool.ready_rx.clone(),
    };

    // Запускаем поток захвата
    spawn_capture_thread(capture_pool, width, height);

    // --- Compute & Tx Thread (Пока просто замеряем FPS) ---
    let mut frames_processed = 0;
    let mut last_report = Instant::now();

    println!("🧠 [Retina] Booting compute loop...");

    loop {
        // 1. Берем готовый кадр
        let frame = match pool.ready_rx.recv() {
            Ok(f) => f,
            Err(_) => break,
        };

        // TODO: Здесь будет Rayon SIMD извлечение фичей и отправка UDP
        // Сейчас просто эмулируем нагрузку препроцессора (1 мс)
        std::thread::sleep(std::time::Duration::from_millis(1));

        // 2. Возвращаем буфер в пул
        pool.free_tx.send(frame).unwrap();

        frames_processed += 1;
        if last_report.elapsed().as_secs() >= 1 {
            println!("👁️ [Retina] Pipeline Speed: {} FPS", frames_processed);
            frames_processed = 0;
            last_report = Instant::now();
        }
    }
}
