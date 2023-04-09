pub mod bme680;
pub mod data_manager;
pub mod net;
pub mod watchdog;

pub(crate) use self::bme680::bme680_task;
pub(crate) use self::data_manager::data_manager_task;
pub(crate) use self::net::{
    eth_gpio_interrupt_handler_task, ipstack_clock_timer_task, ipstack_poll_task,
    ipstack_poll_timer_task,
};
pub(crate) use self::watchdog::watchdog_task;
