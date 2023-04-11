use bme680::{
    Error, FieldData, I2CAddress, IIRFilterSize, OversamplingSetting, PowerMode, SettingsBuilder,
};
use core::fmt;
use stm32f4xx_hal::{
    gpio::{OpenDrain, AF4, AF9, PB10, PB3},
    hal::blocking::{
        delay::DelayMs,
        i2c::{Read, Write},
    },
    i2c::I2c,
    pac::I2C2,
};

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Measurement {
    /// The temperature in centidegress C
    pub temperature: i32,
    /// The relative humidity in centipercent
    pub humidity: u16,
}

pub type DefaultI2cPins = (PB10<AF4<OpenDrain>>, PB3<AF9<OpenDrain>>);
pub type DefaultI2c<PINS = DefaultI2cPins> = I2c<I2C2, PINS>;

pub struct Bme680<D, I2C = DefaultI2c> {
    drv: bme680::Bme680<I2C, D>,
    delay: D,
}

impl<D, I2C> Bme680<D, I2C>
where
    I2C: Read + Write,
    D: DelayMs<u8>,
{
    pub fn new(
        i2c: I2C,
        mut delay: D,
    ) -> Result<Self, Error<<I2C as Read>::Error, <I2C as Write>::Error>> {
        let mut drv = bme680::Bme680::init(i2c, &mut delay, I2CAddress::Secondary)?;
        let settings = SettingsBuilder::new()
            .with_humidity_oversampling(OversamplingSetting::OS2x)
            .with_pressure_oversampling(OversamplingSetting::OS4x)
            .with_temperature_oversampling(OversamplingSetting::OS8x)
            .with_temperature_filter(IIRFilterSize::Size3)
            .build();
        drv.set_sensor_settings(&mut delay, settings)?;
        drv.set_sensor_mode(&mut delay, PowerMode::ForcedMode)?;
        Ok(Self { drv, delay })
    }

    pub fn measure(
        &mut self,
    ) -> Result<Measurement, Error<<I2C as Read>::Error, <I2C as Write>::Error>> {
        self.drv
            .set_sensor_mode(&mut self.delay, PowerMode::ForcedMode)?;
        // TODO does the bme680 driver do the correct delay??
        // https://github.com/boschsensortec/BME68x-Sensor-API/blob/6dab330cb5727006d5046f9eebf357f8909c0ef6/examples/forced_mode/forced_mode.c#L67-L78
        //
        // https://docs.rs/bme680/0.6.0/src/bme680/lib.rs.html#940-996
        //
        //
        let (data, _state) = self.drv.get_sensor_data(&mut self.delay)?;
        Ok(Measurement::from(data))
    }
}

impl From<FieldData> for Measurement {
    fn from(value: FieldData) -> Self {
        Measurement {
            temperature: (value.temperature_celsius() * 100.0) as i32,
            humidity: (value.humidity_percent() * 100.0) as u16,
        }
    }
}

impl fmt::Display for Measurement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "BME680 temperature: {}, humidity: {}",
            self.temperature, self.humidity
        )
    }
}
