use smoltcp::wire::{Ipv4Address, Ipv4Cidr};

pub use self::generated_confg::*;
mod generated_confg {
    include!(concat!(env!("OUT_DIR"), "/generated_config.rs"));
}

pub const IP_CIDR: Ipv4Cidr = Ipv4Cidr::new(Ipv4Address(IP_ADDRESS), 24);

pub const SOCKET_BUFFER_LEN: usize = wire_protocols::broadcast::MESSAGE_LEN * 4;

pub const STARTUP_DELAY_SECONDS: u8 = 5;

pub const WATCHDOG_RESET_PERIOD_MS: u32 = 8000;
pub const WATCHDOG_TASK_INTERVAL_MS: u32 = 1000;

pub const BME680_MEASUREMENT_INTERVAL_MS: u32 = 2500;

/// Number of BCAST_INTERVAL_SEC cycles to wait before starting to send
/// broadcast protocol messages
pub const DATA_MANAGER_WARM_UP_PERIOD_CYCLES: u32 = 24;

pub const BCAST_INTERVAL_SEC: u32 = 5;
