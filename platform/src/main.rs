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
use protobuf::Message;

use crate::bluetooth::ble;
mod bluetooth;

/*
    main is the entry point for the application
    the projects root crate is resposible for implementing hardware specific modules and implementing the main/application ntry point
*/
#[no_mangle]
fn main() {
    esp_idf_sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    // create buffers
    let output_buffer = Arc::new(std::sync::Mutex::new(Vec::<u8>::with_capacity(500)));
    let input_buffer = Arc::new(std::sync::Mutex::new(Vec::<u8>::with_capacity(500)));

    let my_app = app::app::app::App::new(Arc::clone(&input_buffer), Arc::clone(&output_buffer));

    // start and init the bluetooth processing
    let mut bluetooth_processor =
        ble::BluetoothProcessing::new(Arc::clone(&input_buffer), Arc::clone(&output_buffer));
    bluetooth_processor = bluetooth_processor.init_device();
    bluetooth_processor = bluetooth_processor.init_server();

    // run bluetooth processing
    thread::spawn(|| {
        bluetooth_processor.run_ble();
    });

    my_app.run();
}
