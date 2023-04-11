#![deny(warnings, clippy::all)]

use std::env;

fn main() {
    // TODO - just for testing with another device on the network,
    // these are just made up
    env::set_var("AIR_GRADIENT_IP_ADDRESS", "192.168.1.37");
    env::set_var("AIR_GRADIENT_MAC_ADDRESS", "02:00:04:03:06:01");
    env::set_var("AIR_GRADIENT_DEVICE_ID", "4");
    env::set_var("AIR_GRADIENT_BROADCAST_PORT", "32110");

    built::write_built_file().expect("Failed to acquire build-time information");

    env_config::generate_env_config_constants();
}
