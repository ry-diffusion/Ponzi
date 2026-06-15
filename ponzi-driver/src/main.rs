use env_logger::Env;
use log::{error, info};
use signal_hook::{consts::{SIGINT, SIGQUIT, SIGTERM}, iterator::Signals};
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};

use ponzi_driver::config::Config;
use ponzi_driver::protocol::PenData;
use ponzi_driver::usb::Tablet;
use ponzi_driver::virtual_device::{InputProcessor, VirtualKeys, VirtualPen};

const PID_FILE: &str = "/run/ponzi/ponzi.pid";
const CONFIG_PATHS: &[&str] = &["config.toml", "/etc/ponzi/config.toml"];

fn find_config() -> Config {
    for path in CONFIG_PATHS {
        if let Ok(c) = Config::load(Path::new(path)) {
            info!("Loaded config from {path}");
            return c;
        }
    }
    info!("No config found, using defaults");
    Config::default()
}

fn write_pid() {
    let pid = std::process::id();
    if let Some(parent) = Path::new(PID_FILE).parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Err(e) = fs::write(PID_FILE, pid.to_string()) {
        error!("Could not write PID file: {e}");
    } else {
        info!("PID {pid} written to {PID_FILE}");
    }
}

fn remove_pid() {
    let _ = fs::remove_file(PID_FILE);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init_from_env(Env::default().filter_or("RUST_LOG", "info"));

    info!("Ponzi driver v{} starting", env!("CARGO_PKG_VERSION"));

    write_pid();
    let cfg = Arc::new(Mutex::new(find_config()));

    let mut signals = Signals::new([SIGINT, SIGTERM, SIGQUIT])?;

    let (vid, pid) = {
        let c = cfg.lock().unwrap();
        (c.device.vendor_id, c.device.product_id)
    };

    let mut tablet = Tablet::open(vid, pid)?;
    tablet.init()?;
    tablet.set_full_mode()?;
    info!("Tablet attached (VID {vid:04X}, PID {pid:04X})");

    let mut pen = VirtualPen::new(&cfg.lock().unwrap())?;
    let mut keys = VirtualKeys::new(&cfg.lock().unwrap())?;
    let mut processor = InputProcessor::new(Arc::clone(&cfg));
    info!("Virtual input devices registered — driver is running");

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

    remove_pid();
    info!("Driver stopped");
    Ok(())
}
