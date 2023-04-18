use std::{thread, time};

use app::domain::domain;
use bmi160::interface::SpiInterface;
use bmi160::{AccelerometerPowerMode, Bmi160, GyroscopePowerMode, SensorSelector};
use crossbeam_channel::bounded;
use crossbeam_channel::Receiver;
use crossbeam_channel::Sender;
use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::AnyIOPin;
use esp_idf_hal::gpio::Gpio4;
use esp_idf_hal::gpio::Gpio5;
use esp_idf_hal::gpio::Gpio6;
use esp_idf_hal::gpio::Gpio7;
use esp_idf_hal::gpio::Output;
use esp_idf_hal::gpio::PinDriver;
use esp_idf_hal::prelude::*;
use esp_idf_hal::spi::config;
use esp_idf_hal::spi::SpiDeviceDriver;
use esp_idf_hal::spi::{SpiDriver, SPI2};

pub struct BMI160Sensor<'a> {
    imu: Bmi160<SpiInterface<SpiDeviceDriver<'a, SpiDriver<'a>>, PinDriver<'a, Gpio7, Output>>>,
    sender: Sender<domain::BMI160Data>,
    pub receiver: Receiver<domain::BMI160Data>,
}

impl BMI160Sensor<'static> {
    pub fn new(
        _sclk: Gpio6,
        _sdo: Gpio5,
        _sdi: Option<Gpio4>,
        spi2: SPI2,
        cs_gpio: Gpio7,
    ) -> Result<Self, esp_idf_sys::EspError> {
        let config = config::Config::new()
            .baudrate(Hertz(10_000))
            .data_mode(embedded_hal_1::spi::MODE_3)
            .duplex(config::Duplex::Full);

        let cs = PinDriver::output(cs_gpio).unwrap();

        let dev = SpiDeviceDriver::new_single(
            spi2,
            _sclk,
            _sdo,
            _sdi,
            esp_idf_hal::spi::Dma::Disabled,
            Option::<AnyIOPin>::None,
            &config,
        );

        match dev {
            Ok(device) => {
                let (sender, receiver) = bounded::<domain::BMI160Data>(100);
                let imu = Bmi160::new_with_spi(device, cs);
                Ok(BMI160Sensor {
                    imu,
                    sender,
                    receiver,
                })
            }
            Err(err) => {
                ::log::info!("error creating spi device {:?}", err);
                Err(err)
            }
        }
    }
    pub fn init_sensor(&mut self) -> Result<(), esp_idf_sys::EspError> {
        self.imu.chip_id().unwrap();
        self.imu.chip_id().unwrap();
        let id = self.imu.chip_id().unwrap_or_else(|err| {
            ::log::info!("error reading chip id: {:?}", err);
            90
        });
        ::log::info!("Chip ID: {}", id);
        FreeRtos::delay_ms(500);
        ::log::info!("setting power mode!");
        self.imu
            .set_accel_power_mode(AccelerometerPowerMode::Normal)
            .unwrap_or_else(|err| ::log::info!("error setting accel mode: {:?}", err));
        FreeRtos::delay_ms(500);
        ::log::info!("setting gyro power mode!");
        self.imu
            .set_gyro_power_mode(GyroscopePowerMode::Normal)
            .unwrap();
        FreeRtos::delay_ms(500);
        ::log::info!("disabling magnet");
        self.imu
            .set_magnet_power_mode(bmi160::MagnetometerPowerMode::Suspend)
            .unwrap();
        FreeRtos::delay_ms(500);
        ::log::info!("power mode: {:?}", self.imu.power_mode().unwrap());
        FreeRtos::delay_ms(500);
        ::log::info!("status: {:?}", self.imu.status().unwrap());
        FreeRtos::delay_ms(500);
        Ok(())
    }
    pub fn process_loop(&mut self) {
        loop {
            let data = self.imu.data(SensorSelector::all()).unwrap();
            if self.imu.status().unwrap().accel_data_ready
                && self.imu.status().unwrap().gyro_data_ready
            {
                let time = data.time.unwrap();
                let accel = data.accel.unwrap();
                let gyro = data.gyro.unwrap();

                // put data on the channel
                let _ = self.sender.send(domain::BMI160Data {
                    gyro_x: gyro.x,
                    gyro_y: gyro.y,
                    gyro_z: gyro.z,
                    accel_x: accel.x,
                    accel_y: accel.y,
                    accel_z: accel.z,
                    sample_time: time,
                });
            }
        }
    }
}

// // modile
// pub fn init_sensor() {
//     let mut peripherals = Peripherals::take().unwrap();

//     // pins
//     let _sclk = peripherals.pins.gpio6;
//     let _sdo = peripherals.pins.gpio5;
//     let _sdi = Option::Some(peripherals.pins.gpio4);

//     let config = config::Config::new()
//         .baudrate(Hertz(10_000))
//         .data_mode(embedded_hal_1::spi::MODE_3)
//         .duplex(config::Duplex::Full);

//     // let mut gpioRef = unsafe { peripherals.pins.gpio7.clone_unchecked() };
//     // let mut gpioCS = unsafe { esp_idf_hal::gpio::Gpio7::new() };

//     let mut csGPIO = peripherals.pins.gpio7;
//     let cs = PinDriver::output(csGPIO).unwrap();
//     // let chip_select = esp_idf_hal::gpio::Pin::new(4);

//     let mut dev = SpiDeviceDriver::new_single(
//         peripherals.spi2,
//         _sclk,
//         _sdo,
//         _sdi,
//         esp_idf_hal::spi::Dma::Disabled,
//         Option::<AnyIOPin>::None,
//         &config,
//     )
//     .unwrap();

//     let mut imu = Bmi160::new_with_spi(dev, cs);

//     // let mut read = [0u8; 1];
//     // let write = [0x80];

//     imu.chip_id().unwrap();
//     imu.chip_id().unwrap();
//     let id = imu.chip_id().unwrap_or_else(|err| {
//         ::log::info!("error reading chip id: {:?}", err);
//         90
//     });
//     ::log::info!("Chip ID: {}", id);
//     FreeRtos::delay_ms(500);
//     ::log::info!("setting power mode!");
//     imu.set_accel_power_mode(AccelerometerPowerMode::Normal)
//         .unwrap_or_else(|err| ::log::info!("error setting accel mode: {:?}", err));
//     FreeRtos::delay_ms(500);
//     ::log::info!("setting gyro power mode!");
//     imu.set_gyro_power_mode(GyroscopePowerMode::Normal).unwrap();
//     FreeRtos::delay_ms(500);
//     ::log::info!("disabling magnet");
//     imu.set_magnet_power_mode(bmi160::MagnetometerPowerMode::Suspend)
//         .unwrap();
//     FreeRtos::delay_ms(500);
//     ::log::info!("power mode: {:?}", imu.power_mode().unwrap());
//     FreeRtos::delay_ms(500);
//     ::log::info!("status: {:?}", imu.status().unwrap());
//     FreeRtos::delay_ms(500);

//     let mut imu = Bmi160::new_with_spi(dev, cs);
// }
