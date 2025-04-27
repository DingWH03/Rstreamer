pub mod capture;
pub mod server;
pub mod error;

use crate::capture::FrameCapturer;
use crate::server::StreamingServer;
use tokio::sync::broadcast;
use nokhwa::utils::Resolution;

pub struct RStreamer {
    port: u16,
    resolution: Resolution,
    fps: u32,
    device_index: usize,
    capturer: Option<FrameCapturer>,
    server: Option<StreamingServer>,
}

impl RStreamer {
    pub fn builder() -> RStreamerBuilder {
        RStreamerBuilder::default()
    }

    pub async fn start(&mut self) -> error::Result<()> {
        let (sender, receiver) = broadcast::channel(10);
        
        let capturer = FrameCapturer::new(
            self.device_index,
            self.resolution,
            self.fps,
            sender,
        )?;
        
        capturer.start()?;
        self.capturer = Some(capturer); // <-- 保存下来
        let mut server = StreamingServer::new(self.port, receiver)?;
        server.start()?;
        self.server = Some(server);
        
        
        Ok(())
    }
    pub async fn stop(&mut self)-> error::Result<()> {
        if let Some(capturer) = &self.capturer {
            capturer.stop();
        }
        if let Some(server) = &mut self.server {
            server.stop().await?;
        }
        Ok(())
    }
}

#[derive(Default)]
pub struct RStreamerBuilder {
    port: Option<u16>,
    resolution: Option<Resolution>,
    fps: Option<u32>,
    device_index: Option<usize>,
}

impl RStreamerBuilder {
    pub fn port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    pub fn resolution(mut self, width: u32, height: u32) -> Self {
        self.resolution = Some(Resolution::new(width, height));
        self
    }

    pub fn fps(mut self, fps: u32) -> Self {
        self.fps = Some(fps);
        self
    }

    pub fn device_index(mut self, index: usize) -> Self {
        self.device_index = Some(index);
        self
    }

    pub fn build(self) -> RStreamer {
        RStreamer {
            port: self.port.unwrap_or(8080),
            resolution: self.resolution.unwrap_or(Resolution::new(640, 480)),
            fps: self.fps.unwrap_or(30),
            device_index: self.device_index.unwrap_or(0),
            capturer: None,
            server: None,
        }
    }
}