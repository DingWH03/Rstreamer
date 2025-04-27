use clap::Parser;
use rstreamer::{error::Result, RStreamer};
use tracing_subscriber;

#[derive(Parser, Debug)]
#[command(version, about, long_about = "A simple video streamer that captures video from a device.")]
struct Args {
    #[arg(short, long, default_value_t = 8080, help = "Port to bind the server to.")]
    port: u16,

    #[arg(short, long, default_value_t = 0, help = "Device index to capture video from.")]
    device: usize,

    #[arg(
        short, 
        long, 
        default_value = "1280x720", 
        help = "Resolution of the captured video (e.g., 1280x720)."
    )]
    resolution: String,

    #[arg(short, long, default_value_t = 30, help = "Frames per second (FPS) to capture.")]
    fps: u32,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    let (width, height) = parse_resolution(&args.resolution)?;

    let mut streamer = RStreamer::builder()
        .port(args.port)
        .device_index(args.device)
        .resolution(width, height)
        .fps(args.fps)
        .build();

    streamer.start().await?;

    // Listen for Ctrl+C
    tokio::signal::ctrl_c().await?;
    streamer.stop().await?;
    Ok(())
}

fn parse_resolution(res: &str) -> Result<(u32, u32)> {
    let parts: Vec<&str> = res.split('x').collect();
    if parts.len() != 2 {
        return Err(rstreamer::error::RStreamerError::CameraError(
            "Invalid resolution format. Use format WIDTHxHEIGHT (e.g., 1280x720)".into(),
        ));
    }

    // Try to parse both parts as u32, otherwise default to 1280x720
    let width = parts[0].parse().unwrap_or(1280);
    let height = parts[1].parse().unwrap_or(720);

    Ok((width, height))
}
