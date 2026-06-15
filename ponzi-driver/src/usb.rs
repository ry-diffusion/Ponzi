use log::{error, info, warn};
use rusb::{Device, DeviceDescriptor, DeviceHandle, Error as UsbError, GlobalContext, TransferType, devices};
use std::time::Duration;

const TIMEOUT: Duration = Duration::from_secs(1);
const MAX_RETRIES: u8 = 5;

// HID class Set_Report request via control transfer
// bmRequestType: 0x21 = Host-to-device, Class, Interface
// bRequest: 0x09 = SET_REPORT
// wValue: 0x0308 = Report Type (Feature=3), Report ID (0x08)
// wIndex: 2 = Interface number
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
                    Ok(name) => info!("Found device: {}", name),
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
                    error!("Could not read config descriptor {}: {}", i, e);
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
                        .map_err(|e| {
                            error!("Could not claim interface {}: {}", iface_num, e);
                            e
                        })?;
                    self.claimed_interfaces.push(iface_num);
                    info!("Claimed HID interface {}", iface_num);

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
                    info!("Device reset on attempt {}", attempt);
                    return Ok(());
                }
                Err(e) => {
                    error!("Reset attempt {} failed: {}", attempt, e);
                    if attempt == MAX_RETRIES {
                        return Err(e);
                    }
                    std::thread::sleep(TIMEOUT);
                }
            }
        }
        unreachable!()
    }

    /// Send the HID feature report that switches the tablet from Android/partial
    /// mode to full 4096x4096 PC mode.
    pub fn set_full_mode(&self) -> Result<(), UsbError> {
        self.handle.write_control(
            0x21,   // bmRequestType: class, interface, host-to-device
            0x09,   // bRequest: SET_REPORT
            0x0308, // wValue: Feature report, ID 0x08
            2,      // wIndex: interface 2
            &MODESET_REPORT,
            Duration::from_millis(250),
        )?;
        info!("Tablet switched to full PC mode (4096x4096)");
        Ok(())
    }

    pub fn read_input(&self, buf: &mut [u8]) -> Result<usize, UsbError> {
        self.handle.read_interrupt(self.endpoint, buf, TIMEOUT)
    }

    /// Release all claimed HID interfaces back to the kernel.
    /// After this, the kernel usbhid driver re-attaches and apps like
    /// libinput, xf86-input-wacom, or OpenTabletDriver can use the device.
    pub fn release(&mut self) {
        for &iface in &self.claimed_interfaces {
            if let Err(e) = self.handle.release_interface(iface) {
                warn!("Could not release interface {}: {}", iface, e);
            } else {
                info!("Released interface {}", iface);
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
