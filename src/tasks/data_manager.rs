use crate::{app::data_manager_task, config, sensors::bme680, util};
use log::{debug, warn};
use smoltcp::{socket::udp::Socket as UdpSocket, wire::Ipv4Address};
use stm32f4xx_hal::prelude::*;
use wire_protocols::{
    broadcast::{Message as WireMessage, Repr as Message},
    DateTime, DeviceSerialNumber, ProtocolVersion, StatusFlags,
};

const LOCAL_EPHEMERAL_PORT: u16 = 16000;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum SpawnArg {
    /// Temperature and humidity measurement from the BME680 sensor
    Bme680Measurement(bme680::Measurement),
    /// Time to send the broadcast protocol data
    SendBroadcastMessage,
}

pub struct TaskState {
    msg: Message,
    cycles_till_warmed_up: u32,
}

impl TaskState {
    pub const fn new() -> Self {
        Self {
            msg: default_bcast_message(),
            cycles_till_warmed_up: config::DATA_MANAGER_WARM_UP_PERIOD_CYCLES,
        }
    }
}

// TODO - state management, rtc, status bits, timeout/invalidate, etc
// add a warm up period before starting the broadcast protocol
// make SystemStatus msg sn Option to indicate it on display too
pub(crate) fn data_manager_task(ctx: data_manager_task::Context, arg: SpawnArg) {
    let state = ctx.local.state;
    let sockets = ctx.shared.sockets;
    let udp_socket_handle = ctx.shared.udp_socket;

    let socket = sockets.get_mut::<UdpSocket>(*udp_socket_handle);

    if !state.msg.status_flags.initialized() {
        debug!("DM: initializing data manager state");
        state.msg.device_serial_number = util::read_device_serial_number();
        state.msg.status_flags.set_initialized(true);
    }

    let mut send_msg = false;
    match arg {
        SpawnArg::Bme680Measurement(m) => {
            state.msg.temperature = m.temperature;
            state.msg.humidity = m.humidity;
            state.msg.status_flags.set_temperature_valid(true);
            state.msg.status_flags.set_humidity_valid(true);
        }
        SpawnArg::SendBroadcastMessage => {
            // TODO invalidate stale fields on timer or keep valid?

            if state.cycles_till_warmed_up != 0 {
                state.cycles_till_warmed_up = state.cycles_till_warmed_up.saturating_sub(1);

                if state.cycles_till_warmed_up == 0 {
                    debug!("DM: warm up period complete");
                }
            } else {
                send_msg = true;
            }

            state.msg.uptime_seconds += config::BCAST_INTERVAL_SEC;

            data_manager_task::spawn_after(
                config::BCAST_INTERVAL_SEC.secs(),
                SpawnArg::SendBroadcastMessage,
            )
            .unwrap();
        }
    }

    if send_msg {
        if !socket.is_open() {
            socket.bind(LOCAL_EPHEMERAL_PORT).unwrap();
        }

        if socket.can_send() {
            match socket.send(
                state.msg.message_len(),
                (
                    Ipv4Address(config::BROADCAST_ADDRESS),
                    config::BROADCAST_PORT,
                )
                    .into(),
            ) {
                Err(e) => warn!("Failed to send. {e:?}"),
                Ok(buf) => {
                    let mut wire = WireMessage::new_unchecked(buf);
                    state.msg.emit(&mut wire);
                    //debug!("DM: Sent message sn {}", state.msg.sequence_number);
                    state.msg.sequence_number = state.msg.sequence_number.wrapping_add(1);
                }
            }
        } else {
            warn!("Socket cannot send");
            socket.close();
        }
    }
}

const fn default_bcast_message() -> Message {
    Message {
        protocol_version: ProtocolVersion::v1(),
        firmware_version: config::FIRMWARE_VERSION,
        device_id: config::DEVICE_ID,
        device_serial_number: DeviceSerialNumber::zero(),
        sequence_number: 0,
        uptime_seconds: 0,
        status_flags: StatusFlags::empty(),
        datetime: DateTime::zero(),
        temperature: 0,
        humidity: 0,
        voc_ticks: 0,
        nox_ticks: 0,
        voc_index: 0,
        nox_index: 0,
        pm2_5_atm: 0,
        co2: 0,
    }
}
