// src/frame_capturer.rs

use nokhwa::{
    Camera, pixel_format::RgbFormat,
    utils::{ApiBackend, CameraFormat, CameraIndex, FrameFormat, RequestedFormat, RequestedFormatType, Resolution},
};
use std::{sync::{atomic::{AtomicBool, Ordering}, Arc}, thread, time::Duration};
use tokio::{sync::broadcast, task::JoinHandle};
use crate::error::RStreamerError;

pub struct FrameCapturer {
    device_index: usize,
    resolution: Resolution,
    fps: u32,
    sender: broadcast::Sender<Vec<u8>>,
    running: Arc<AtomicBool>,
}

impl FrameCapturer {
    pub fn new(
        device_index: usize,
        resolution: Resolution,
        fps: u32,
        sender: broadcast::Sender<Vec<u8>>,
    ) -> Result<Self, RStreamerError> {
        Ok(Self { 
            device_index,
            resolution,
            fps,
            sender,
            running: Arc::new(AtomicBool::new(true)),
        })
    }

    pub fn start(&self) -> Result<JoinHandle<()>, RStreamerError> {
        let sender = self.sender.clone();
        let device_index = self.device_index;
        let resolution = self.resolution.clone();
        let fps = self.fps;
        let running = self.running.clone();

        let handle = tokio::task::spawn_blocking(move || {
            let camera_index = match device_index.try_into() {
                Ok(idx) => CameraIndex::Index(idx),
                Err(_) => {
                    tracing::error!("Invalid device index: {}", device_index);
                    return;
                }
            };

            let requested_format = RequestedFormat::new::<RgbFormat>(
                RequestedFormatType::Closest(CameraFormat::new(
                    resolution,
                    FrameFormat::MJPEG,
                    fps,
                ))
            );

            let mut camera = match Camera::with_backend(
                camera_index,
                requested_format,
                ApiBackend::Video4Linux
            ) {
                Ok(mut cam) => {
                    if let Err(e) = cam.open_stream() {
                        tracing::error!("Failed to start camera stream: {}", e);
                        return;
                    }
                    cam
                }
                Err(e) => {
                    tracing::error!("Camera initialization failed: {}", e);
                    return;
                }
            };

            if !camera.is_stream_open() {
                tracing::error!("Camera stream not open after initialization");
                return;
            }

            let black_frame = vec![0u8; (resolution.width() * resolution.height() * 3) as usize];

            'capture_loop: loop {
                if !running.load(Ordering::Relaxed) {
                    break 'capture_loop;
                }

                match camera.frame() {
                    Ok(frame) => {
                        let buf = frame.buffer().to_vec();
                        if sender.send(buf).is_err() {
                            tracing::warn!("Receiver disconnected, stopping capture");
                            break 'capture_loop;
                        }
                    }
                    Err(e) => {
                        tracing::error!("Frame capture error: {}, sending black frame", e);
                        if sender.send(black_frame.clone()).is_err() {
                            tracing::warn!("Receiver disconnected, stopping capture");
                            break 'capture_loop;
                        }

                        thread::sleep(Duration::from_secs(1));
                        if let Err(e) = camera.open_stream() {
                            tracing::error!("Failed to restart stream: {}, will retry", e);
                        }
                    }
                }

                thread::sleep(Duration::from_millis(1000 / fps as u64));
            }

            if let Err(e) = camera.stop_stream() {
                tracing::error!("Failed to stop stream: {}", e);
            }
        });

        Ok(handle)
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }
}
