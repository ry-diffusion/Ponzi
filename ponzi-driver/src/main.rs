use env_logger::Env;
use log::info;
use signal_hook::{consts::{SIGINT, SIGQUIT, SIGTERM}, iterator::Signals};
use std::path::Path;

use ponzi_driver::config::Config;
use ponzi_driver::protocol::PenData;
use ponzi_driver::usb::Tablet;
use ponzi_driver::virtual_device::{InputProcessor, VirtualKeys, VirtualPen};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init_from_env(Env::default().filter_or("RUST_LOG", "info"));

    let cfg = match Config::load(Path::new("config.toml")) {
        Ok(c) => {
            info!("Loaded config.toml");
            c
        }
        Err(e) => {
            info!("No config.toml ({}), using defaults", e);
            Config::default()
        }
    };

    let mut signals = Signals::new([SIGINT, SIGTERM, SIGQUIT])?;

    let mut tablet = Tablet::open(cfg.device.vendor_id, cfg.device.product_id)?;
    tablet.init()?;
    tablet.set_full_mode()?;
    info!("Tablet attached and running");

    let mut pen = VirtualPen::new(&cfg.pressure, &cfg.mapping)?;
    let mut keys = VirtualKeys::new(&cfg.buttons)?;
    let mut processor = InputProcessor::new(&cfg);
    info!("Virtual devices ready");

    let mut buf = vec![0u8; 64];

    loop {
        if signals.pending().next().is_some() {
            info!("Signal received, shutting down");
            break;
        }

        if tablet.read_input(&mut buf).is_ok() {
            let data = PenData::decode(&buf);
            processor.process(&data, &mut pen, &mut keys);
        }
    }

    Ok(())
}
