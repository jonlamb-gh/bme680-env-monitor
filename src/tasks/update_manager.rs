use crate::{app::update_manager_task, config, util};
use bootloader_lib::UpdateConfigAndStatus;
use log::{debug, warn};
use smoltcp::socket::tcp::{self, Socket as TcpSocket, State};
use stm32f4xx_hal::prelude::*;
use update_manager::{ActionToTake, DeviceInfo, UpdateManager};
use wire_protocols::device::DEFAULT_PORT;

pub struct TaskState {
    um: UpdateManager,
}

impl TaskState {
    pub const fn new() -> Self {
        Self {
            // TODO config for port
            um: UpdateManager::new(DEFAULT_PORT),
        }
    }
}

pub(crate) fn update_manager_task(ctx: update_manager_task::Context) {
    let state = ctx.local.state;
    let device_info = ctx.local.device_info;
    let sockets = ctx.shared.sockets;
    let socket_handle = ctx.shared.device_socket;

    let socket = sockets.get_mut::<TcpSocket>(*socket_handle);

    // TODO
    if let Some(action_to_take) = state.um.update(device_info, socket).unwrap() {
        match action_to_take {
            ActionToTake::Reboot => {
                warn!("Rebooting now");
                unsafe { bootloader_lib::sw_reset() };
            }
            ActionToTake::CompleteAndReboot => {
                warn!("Update complete, rebooting now");
                UpdateConfigAndStatus::set_update_pending();
                unsafe { bootloader_lib::sw_reset() };
            }
        }
    }

    update_manager_task::spawn_after(config::UPDATE_MANAGER_POLL_INTERVAL_MS.millis()).unwrap();
}
