use smoltcp::wire::{Ipv4Address, Ipv4Cidr};

pub use self::generated_confg::*;
#[allow(dead_code)]
mod generated_confg {
    include!(concat!(env!("OUT_DIR"), "/env_config.rs"));
}

pub const IP_CIDR: Ipv4Cidr = Ipv4Cidr::new(Ipv4Address(IP_ADDRESS), 24);

pub const SOCKET_BUFFER_LEN: usize = wire_protocols::broadcast::MESSAGE_LEN * 4;

pub const WATCHDOG_RESET_PERIOD_MS: u32 = 8000;
pub const WATCHDOG_TASK_INTERVAL_MS: u32 = 1000;
