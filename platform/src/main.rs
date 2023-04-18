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
use esp_idf_hal::{prelude::Peripherals, task::thread::ThreadSpawnConfiguration};
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

    let button_result = drivers::button::Button::new();

    let lsm303_result = drivers::lsm303agr::new(
        peripherals.pins.gpio0,
        peripherals.pins.gpio1,
        peripherals.i2c0,
    );
    let mut lsm303 = lsm303_result.unwrap();

    // ::log::info!("error initialising lsm303agr {:?}", err)
    // pins
    let _cs_gpio = peripherals.pins.gpio7;
    let _sclk = peripherals.pins.gpio6;
    let _sdo = peripherals.pins.gpio5;
    let _sdi = Option::Some(peripherals.pins.gpio4);

    let bmi160_result =
        drivers::bmi160_sensor::BMI160Sensor::new(_sclk, _sdo, _sdi, peripherals.spi2, _cs_gpio);
    let mut bmi160 = bmi160_result.unwrap();
    bmi160.init_sensor();

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

    ThreadSpawnConfiguration {
        name: Some("Thread-H\0".as_bytes()),
        priority: 5,
        ..Default::default()
    }
    .set()
    .unwrap();
    let _ = thread::Builder::new().spawn(|| {
        bluetooth_processor.run_ble();
    });

    /*
        Init and Run App
    */
    let button = Box::new(button_result.unwrap());

    let my_app = app::app::app::App::new(
        Arc::clone(&input_buffer),
        Arc::clone(&output_buffer),
        button.button_channel(),
        lsm303.receiver.clone(),
        bmi160.receiver.clone(),
    );

    // start hardware tasks
    ThreadSpawnConfiguration {
        name: Some("Thread-A\0".as_bytes()),
        priority: 5,
        ..Default::default()
    }
    .set()
    .unwrap();
    thread::Builder::new().spawn(|| {
        button.button_loop();
    });

    ThreadSpawnConfiguration {
        name: Some("Thread-B\0".as_bytes()),
        priority: 25,
        ..Default::default()
    }
    .set()
    .unwrap();
    thread::Builder::new().spawn(move || {
        lsm303.process_loop();
    });

    ThreadSpawnConfiguration {
        name: Some("Thread-C\0".as_bytes()),
        priority: 25,
        ..Default::default()
    }
    .set()
    .unwrap();
    thread::Builder::new().spawn(move || {
        bmi160.process_loop();
    });

    // run app tasks
    my_app.run();
}
