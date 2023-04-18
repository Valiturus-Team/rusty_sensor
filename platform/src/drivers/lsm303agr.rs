use core::time;
use std::{fmt::Debug, thread};

use app::domain::domain;
use crossbeam_channel::{bounded, unbounded, Receiver, Sender};
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
    sender: Sender<domain::LSMData>,
    pub receiver: Receiver<domain::LSMData>,
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

    let (sender, receiver) = unbounded();

    match continuos_mag_result {
        Ok(sensor) => Ok(LSM303AGRSensor {
            sensor,
            sender,
            receiver,
        }),
        Err(_) => Err(Error::HardwareError),
    }
}

impl LSM303AGRSensor<I2cInterface<I2cDriver<'static>>> {
    pub fn process_loop(&mut self) {
        self.sensor.init().unwrap();
        self.sensor
            .set_accel_odr(AccelOutputDataRate::Hz400)
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
                let acell_data = self.sensor.accel_data().unwrap();
                // let mag_data = self.sensor.mag_data().unwrap();
                // println!(
                //     "{:?},{:?},{:?},{:?},{:?},{:?},{:?}",
                //     acell_data.x,
                //     acell_data.y,
                //     acell_data.z,
                //     { esp_timer_get_time() },
                // );

                // if the channel is slow we could try chunnking the channel
                let _ = self.sender.send(domain::LSMData {
                    mag_x: 0,
                    mag_y: 0,
                    mag_z: 0,
                    accel_x: acell_data.x,
                    accel_y: acell_data.y,
                    accel_z: acell_data.z,
                    timestamp: unsafe { esp_timer_get_time() },
                });
            }
        }
    }
}
