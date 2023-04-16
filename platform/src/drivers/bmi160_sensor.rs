use bmi160::interface;
use bmi160::interface::ReadData;
use bmi160::interface::SpiInterface;
use bmi160::Sensor3DData;
use embedded_hal_1::spi::SpiDevice;
use esp_idf_hal::gpio::AnyOutputPin;
use esp_idf_hal::gpio::Gpio4;
use esp_idf_hal::gpio::Gpio5;
use esp_idf_hal::gpio::Gpio6;
use esp_idf_hal::prelude::*;
use std::any::Any;
use std::sync::Arc;

use bmi160::{AccelerometerPowerMode, Bmi160, GyroscopePowerMode, SensorSelector, SlaveAddr};

use embedded_hal::spi::MODE_1;
use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio;
use esp_idf_hal::gpio::AnyIOPin;
use esp_idf_hal::gpio::Gpio7;
use esp_idf_hal::gpio::Output;
use esp_idf_hal::gpio::PinDriver;
use esp_idf_hal::peripheral::Peripheral;
use esp_idf_hal::spi::config;
use esp_idf_hal::spi::SpiAnyPins;
use esp_idf_hal::spi::SpiDeviceDriver;
use esp_idf_hal::spi::SpiSoftCsDeviceDriver;
use esp_idf_hal::{
    peripherals::Peripherals,
    prelude::*,
    spi::{SpiConfig, SpiDriver, SPI2},
};
use esp_idf_sys::esp_log_write;
use esp_idf_sys::EspError;

pub struct BMI160Sensor<'a> {
    imu: Bmi160<SpiInterface<SpiDeviceDriver<'a, SpiDriver<'a>>, PinDriver<'a, Gpio7, Output>>>,
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
                let imu = Bmi160::new_with_spi(device, cs);
                Ok(BMI160Sensor { imu })
            }
            Err(err) => {
                println!("error creating spi device {:?}", err);
                Err(err)
            }
        }
    }
    pub fn init_sensor() -> Result<(), esp_idf_sys::EspError> {
        Ok(())
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
//         println!("error reading chip id: {:?}", err);
//         90
//     });
//     println!("Chip ID: {}", id);
//     FreeRtos::delay_ms(500);
//     println!("setting power mode!");
//     imu.set_accel_power_mode(AccelerometerPowerMode::Normal)
//         .unwrap_or_else(|err| println!("error setting accel mode: {:?}", err));
//     FreeRtos::delay_ms(500);
//     println!("setting gyro power mode!");
//     imu.set_gyro_power_mode(GyroscopePowerMode::Normal).unwrap();
//     FreeRtos::delay_ms(500);
//     println!("disabling magnet");
//     imu.set_magnet_power_mode(bmi160::MagnetometerPowerMode::Suspend)
//         .unwrap();
//     FreeRtos::delay_ms(500);
//     println!("power mode: {:?}", imu.power_mode().unwrap());
//     FreeRtos::delay_ms(500);
//     println!("status: {:?}", imu.status().unwrap());
//     FreeRtos::delay_ms(500);

//     let mut imu = Bmi160::new_with_spi(dev, cs);
// }
