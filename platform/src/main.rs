#![no_main]
#![feature(core_intrinsics)]
#![feature(deadline_api)]

extern crate alloc;

use core::time;
use std::{
    fs::File,
    io::{BufReader, BufWriter, Read, Write},
    sync::Arc,
    thread,
};

use app::domain::domain::{self, ReadWrite};
use crossbeam_channel::unbounded;
use drivers::button::{self};
use esp_idf_hal::prelude::Peripherals;
use protobuf::Message;

use crate::bluetooth::ble;
mod bluetooth;
mod drivers;

/*
    main is the entry point for the application
    the projects root crate is resposible for implementing hardware specific modules and implementing the main/application ntry point
*/
#[no_mangle]
fn main() {
    esp_idf_sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    /*
        init hardware
    */
    let peripherals = Peripherals::take().unwrap();

    let buttonResult = drivers::button::Button::new();

    let lsm303 = drivers::lsm303agr::new(
        peripherals.pins.gpio0,
        peripherals.pins.gpio1,
        peripherals.i2c0,
    );
    // println!("error initialising lsm303agr {:?}", err)
    // pins
    let _cs_gpio = peripherals.pins.gpio7;
    let _sclk = peripherals.pins.gpio6;
    let _sdo = peripherals.pins.gpio5;
    let _sdi = Option::Some(peripherals.pins.gpio4);

    let bmi160_result =
        drivers::bmi160_sensor::BMI160Sensor::new(_sclk, _sdo, _sdi, peripherals.spi2, _cs_gpio);
    // println!("error initialising bmi160 {:?}",err);

    /*
        Init and Run Bluetotooth
    */
    // create buffersfor between bluetooth and app
    let output_buffer = Arc::new(std::sync::Mutex::new(Vec::<u8>::with_capacity(500)));
    let input_buffer = Arc::new(std::sync::Mutex::new(Vec::<u8>::with_capacity(500)));

    let mut bluetooth_processor =
        ble::BluetoothProcessing::new(Arc::clone(&input_buffer), Arc::clone(&output_buffer));
    bluetooth_processor = bluetooth_processor.init_device();
    bluetooth_processor = bluetooth_processor.init_server();

    thread::spawn(|| {
        bluetooth_processor.run_ble();
    });

    /*
        Init and Run App
    */
    let button = Box::new(buttonResult.unwrap());

    let my_app = app::app::app::App::new(
        Arc::clone(&input_buffer),
        Arc::clone(&output_buffer),
        button.button_channel(),
    );

    // start hardware tasks
    thread::spawn(|| {
        button.button_loop();
    });

    // run app tasks
    my_app.run();
}
