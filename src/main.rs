mod drivers;

use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported

use drivers::button;

fn main() {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_sys::link_patches();

    println!("Hello, world!");

    // application
    let my_application = Box::new(App {});

    /*
        Driver hardware inits
    */
    button::init_button();

    // may needs threads and mutexes to share data

    /*
        Hardware loops
    */
    button::button_loop(my_application);
}

/*
    application implementation (hardware agnostic :)
*/
struct App {}

impl button::ButtonActioner for App {
    fn on_pressed(&self) {
        println!("a button has been pressed!");
    }
}
