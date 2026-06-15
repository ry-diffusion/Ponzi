use log::{error, info, warn};
use rusb::{Device, DeviceDescriptor, DeviceHandle, Error as UsbError, GlobalContext, TransferType, devices};
use std::time::Duration;

const TIMEOUT: Duration = Duration::from_secs(1);
const MAX_RETRIES: u8 = 5;

// Type 3: active area modeset — sets coordinate range to full 4096x4096
const MODESET_REPORT: [u8; 8] = [0x08, 0x03, 0x00, 0xFF, 0xF0, 0x00, 0xFF, 0xF0];

pub struct Tablet {
    device: Device<GlobalContext>,
    handle: DeviceHandle<GlobalContext>,
    descriptor: DeviceDescriptor,
    endpoint: u8,
    claimed_interfaces: Vec<u8>,
}

impl Tablet {
    pub fn open(vid: u16, pid: u16) -> Result<Self, UsbError> {
        for device in devices()?.iter() {
            let descriptor = device.device_descriptor()?;
            if descriptor.vendor_id() == vid && descriptor.product_id() == pid {
                let handle = device.open()?;

                match handle.read_product_string_ascii(&descriptor) {
                    Ok(name) => info!("Found device: {name}"),
                    Err(_) => warn!("Found device but could not read product name"),
                }

                return Ok(Self {
                    device,
                    handle,
                    descriptor,
                    endpoint: 0,
                    claimed_interfaces: Vec::new(),
                });
            }
        }
        Err(UsbError::NoDevice)
    }

    pub fn init(&mut self) -> Result<(), UsbError> {
        self.handle.set_auto_detach_kernel_driver(true)?;
        self.claim_hid_endpoint()?;
        self.reset_device()?;
        Ok(())
    }

    fn claim_hid_endpoint(&mut self) -> Result<(), UsbError> {
        let mut found_endpoint = false;

        for i in 0..self.descriptor.num_configurations() {
            let config = match self.device.config_descriptor(i) {
                Ok(c) => c,
                Err(e) => {
                    error!("Could not read config descriptor {i}: {e}");
                    continue;
                }
            };

            for interface in config.interfaces() {
                for desc in interface.descriptors() {
                    if desc.class_code() != rusb::constants::LIBUSB_CLASS_HID {
                        continue;
                    }

                    // Claim ALL HID interfaces (both the pen data iface and the modeset iface)
                    let iface_num = desc.interface_number();
                    self.handle.claim_interface(iface_num)
                        .inspect_err(|&e| {
                            error!("Could not claim interface {iface_num}: {e}");
                        })?;
                    self.claimed_interfaces.push(iface_num);
                    info!("Claimed HID interface {iface_num}");

                    for ep in desc.endpoint_descriptors() {
                        if ep.transfer_type() == TransferType::Interrupt && ep.max_packet_size() == 64 {
                            self.endpoint = ep.address();
                            info!("Using endpoint 0x{:02X} on interface {}", self.endpoint, desc.interface_number());
                            found_endpoint = true;
                        }
                    }
                }
            }
        }

        if !found_endpoint {
            error!("No suitable HID interrupt endpoint found (need 64-byte max packet)");
            return Err(UsbError::NotFound);
        }
        Ok(())
    }

    fn reset_device(&mut self) -> Result<(), UsbError> {
        for attempt in 1..=MAX_RETRIES {
            match self.handle.reset() {
                Ok(()) => {
                    info!("Device reset on attempt {attempt}");
                    return Ok(());
                }
                Err(e) => {
                    error!("Reset attempt {attempt} failed: {e}");
                    if attempt == MAX_RETRIES {
                        return Err(e);
                    }
                    std::thread::sleep(TIMEOUT);
                }
            }
        }
        unreachable!()
    }

    fn send_feature_report(&self, report: &[u8]) -> Result<(), UsbError> {
        self.handle.write_control(0x21, 0x09, 0x0308, 2, report, Duration::from_millis(250))?;
        Ok(())
    }

    pub fn set_full_mode(&self) -> Result<(), UsbError> {
        self.send_feature_report(&MODESET_REPORT)?;
        info!("Sent modeset report (type 3: 4096x4096 active area)");
        Ok(())
    }

    pub fn read_input(&self, buf: &mut [u8]) -> Result<usize, UsbError> {
        self.handle.read_interrupt(self.endpoint, buf, TIMEOUT)
    }

    /// Release all claimed HID interfaces back to the kernel.
    /// After this, the kernel usbhid driver re-attaches and apps like
    /// libinput, xf86-input-wacom, or `OpenTabletDriver` can use the device.
    pub fn release(&mut self) {
        for &iface in &self.claimed_interfaces {
            if let Err(e) = self.handle.release_interface(iface) {
                warn!("Could not release interface {iface}: {e}");
            } else {
                info!("Released interface {iface}");
            }
        }
        self.claimed_interfaces.clear();
        // Attach kernel driver back so usbhid picks it up
        for iface in [1u8, 2] {
            let _ = self.handle.attach_kernel_driver(iface);
        }
        info!("Tablet unlocked for other applications");
    }

    pub fn is_claimed(&self) -> bool {
        !self.claimed_interfaces.is_empty()
    }
}
