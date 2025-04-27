# Rstreamer

一个推流mjpeg到浏览器的小demo。

## 作为CLI工具运行

```bash
cargo run -- --device 0 --port 8080 --resolution 1280x720 --fps 30
```

## 作为库使用

```rust
use rstreamer::RStreamer;

#[tokio::main]
async fn main() -> Result<()> {
    let mut streamer = RStreamer::builder()
        .port(args.port)
        .device_index(args.device)
        .resolution(width, height)
        .fps(args.fps)
        .build();

    streamer.start().await?;
    }
```
