use std::fmt::Debug;

use app::domain::domain;
use esp_idf_hal::{
    gpio::{Gpio0, Gpio1},
    i2c::{config, I2cDriver, I2C0},
    peripherals::Peripherals,
    prelude::*,
};

use esp_idf_sys::esp_timer_get_time;

// If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use lsm303agr::{
    interface::I2cInterface, mode::MagContinuous, AccelOutputDataRate, Lsm303agr, Measurement,
};

#[derive(Debug)]
pub struct LSM303AGRSensor<I2c> {
    sensor: Lsm303agr<I2c, MagContinuous>,
}

#[derive(Debug)]
pub enum Error {
    HardwareError,
}

/*
    Init sensor
*/
pub fn new(
    sda: Gpio0,
    scl: Gpio1,
    _i2c: I2C0,
) -> Result<LSM303AGRSensor<I2cInterface<I2cDriver<'static>>>, Error> {
    // master configuration (default)
    let i2c_config = config::Config {
        baudrate: Hertz(500000),
        sda_pullup_enabled: false,
        scl_pullup_enabled: false,
    };

    let res = I2cDriver::new(_i2c, sda, scl, &i2c_config);

    if res.is_err() {
        return Err(Error::HardwareError);
    }

    let sensor = Lsm303agr::new_with_i2c(res.unwrap());

    let continuos_mag_result = sensor.into_mag_continuous();

    match continuos_mag_result {
        Ok(res) => {
            return Ok(LSM303AGRSensor { sensor: res });
        }
        Err(_) => Err(Error::HardwareError),
    }
}

impl LSM303AGRSensor<I2cInterface<I2cDriver<'static>>> {
    pub fn button_loop(&mut self, reader: Box<dyn LSM303agrReader>) {
        self.sensor.init().unwrap();
        self.sensor
            .set_accel_odr(AccelOutputDataRate::Hz100)
            .unwrap();
        self.sensor
            .set_mag_odr(lsm303agr::MagOutputDataRate::Hz100)
            .unwrap();

        loop {
            // instead of checking the status we could set max read rate
            // this should be faster than the odr of each sensor
            // then we dont check status
            // then we dont need seprate threads (beccause accell_status is blocking and so)

            if self.sensor.accel_status().unwrap().xyz_new_data {
                let data = self.sensor.accel_data().unwrap();
                reader.read_accel_data(data, micros());
            }
            if self.sensor.mag_status().unwrap().xyz_new_data {
                let data = self.sensor.mag_data().unwrap();
                reader.read_mag_data(data, micros());
            }
        }
    }
}

fn micros() -> i64 {
    unsafe { esp_timer_get_time() }
}

pub trait LSM303agrReader {
    fn read_mag_data(&self, measurement: Measurement, microsTimestamp: i64);
    fn read_accel_data(&self, measurement: Measurement, microsTimestamp: i64);
}
