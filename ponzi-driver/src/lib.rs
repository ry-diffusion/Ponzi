#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::module_name_repetitions,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::too_many_lines,
    clippy::similar_names,
    clippy::needless_pass_by_value,
    clippy::trivially_copy_pass_by_ref,
)]

pub mod config;
pub mod protocol;
pub mod usb;
pub mod virtual_device;
