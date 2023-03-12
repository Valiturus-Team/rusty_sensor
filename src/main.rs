mod drivers;

use drivers::button;

use drivers::lsm303agr;
use drivers::lsm303agr::LSM303agrReader;

fn main() {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_sys::link_patches();

    let mut appBox = Box::new(App {});

    // lsm sensor always 100HZ for now
    let lsmSensor = lsm303agr::init_sensor();

    match lsmSensor {
        Ok(mut sensor) => {
            sensor.button_loop(appBox);
        }
        Err(err) => println!("Error initing sensor {:?}", err),
    }
}

/*
    application implementation (hardware agnostic :)
*/
struct App {}

// using an app for everything isnt going to work
// we are going to need to set up some channels etc

impl button::ButtonActioner for App {
    fn on_pressed(&self) {
        println!("a button has been pressed!");
    }
}

impl LSM303agrReader for App {
    // maybe theese should have millisecond timestamps from the device so we know when measurements were taken!
    fn read_mag_data(&self, measurement: ::lsm303agr::Measurement, micros_timestamp: i64) {
        println!(
            "recvd magnetometer data: x {} y {} z {} at {}",
            measurement.x, measurement.y, measurement.z, micros_timestamp
        );
    }
    fn read_accel_data(&self, measurement: ::lsm303agr::Measurement, micros_timestamp: i64) {
        println!(
            "recvd accell data: x {} y {} z {} at {}",
            measurement.x, measurement.y, measurement.z, micros_timestamp
        );
    }
}
