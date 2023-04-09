use crate::{
    app::{bme680_task, data_manager_task},
    config,
    tasks::data_manager::SpawnArg as DataManagerSpawnArg,
};
use log::debug;
use stm32f4xx_hal::prelude::*;

pub(crate) fn bme680_task(ctx: bme680_task::Context) {
    let sensor = ctx.local.bme680;
    let measurement = sensor.measure().unwrap();
    debug!("{measurement}");

    data_manager_task::spawn(DataManagerSpawnArg::Bme680Measurement(measurement)).unwrap();
    bme680_task::spawn_after(config::BME680_MEASUREMENT_INTERVAL_MS.millis()).unwrap();
}
