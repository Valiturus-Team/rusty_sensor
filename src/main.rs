#![no_main]
#![feature(core_intrinsics)]

extern crate alloc;

use std::{
    any::Any,
    ffi::c_void,
    intrinsics::size_of,
    ops::Index,
    ptr,
    sync::{
        mpsc::{channel, sync_channel},
        Arc,
    },
};

use alloc::format;
use esp32_nimble::{utilities::mutex::Mutex, uuid128, BLEDevice, NimbleProperties};

use esp_idf_sys::{
    self as _, esp_ota_end, esp_ota_handle_t, esp_ota_set_boot_partition, esp_ota_write,
    esp_partition_t,
};

enum apply_update_values {
    NONE_ATTEMPTED,
}

// let a = 0;
// global update handle
// let mut updateHandle = esp_ota_handle_t::from;
static mut updateHandle: esp_ota_handle_t = 0;

const NONE_ATTEMPTED: u8 = 2;
const UPDATE_FAIL: u8 = 1;
const UPDATE_SUCCESS: u8 = 0;

const OTA_CONTROL_NOP: u8 = 0x00;
const OTA_CONTROL_REQUEST: u8 = 0x01;
const OTA_CONTROL_REQUEST_ACK: u8 = 0x02;
const OTA_CONTROL_REQUEST_NAK: u8 = 0x03;
const OTA_CONTROL_DONE: u8 = 0x04;
const OTA_CONTROL_DONE_ACK: u8 = 0x05;
const OTA_CONTROL_DONE_NAK: u8 = 0x06;

#[no_mangle]
fn main() {
    esp_idf_sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let ble_device = BLEDevice::take();

    ble_device
        .set_power(
            esp32_nimble::enums::PowerType::Default,
            esp32_nimble::enums::PowerLevel::P3,
        )
        .unwrap();

    let server = ble_device.get_server();
    server.on_connect(|_| {
        ::log::info!("Client connected");
    });

    server.on_disconnect(|_| {
        ::log::info!("start advertising again, client disconnected");
        ble_device.get_advertising().start().unwrap_or_else(|_| {
            ::log::info!("error starting advertising");
        });
    });

    let (sender, receiver) = sync_channel::<BluetoothOperation>(5);
    let sender2 = sender.clone();
    let sender3 = sender.clone();

    let ota_logic = Arc::new(Mutex::new(OtaLogic {
        update_available: false,
        updating: false,
        packet_size: 0,
        packets_received: 0,
    }));

    // Create Arcs for any thread needing to write to ota_logic
    let ota_logic_write_thread = Arc::clone(&ota_logic);
    let ota_logic_apply_update_thread = Arc::clone(&ota_logic);
    let ota_logic_control_char_thread = Arc::clone(&ota_logic);
    let ota_logic_data_thread = Arc::clone(&ota_logic);

    /* ota service */

    let ota_service = server.create_service(uuid128!("d6f1d96d-594c-4c53-b1c6-244a1dfde6d8"));

    let ota_control_characteristic = ota_service.lock().create_characteristic(
        uuid128!("7ad671aa-21c0-46a4-b722-270e3ae3d830"),
        NimbleProperties::READ | NimbleProperties::WRITE | NimbleProperties::NOTIFY,
    );

    ota_control_characteristic.lock().on_write(move |value, _| {
        if value.is_empty() {
            return; // return early if char value is null
        }

        // if we have received a write signal (1) to write
        match *value.index(0) {
            OTA_CONTROL_REQUEST => {
                ::log::info!("we have been told to write the file");
                unsafe {
                    let partition =
                        esp_idf_sys::esp_ota_get_next_update_partition(std::ptr::null());
                    if partition.is_null() {
                        ::log::error!("no ota partition");
                    }
                    let error = esp_idf_sys::esp_ota_begin(
                        partition,
                        esp_idf_sys::OTA_WITH_SEQUENTIAL_WRITES as usize,
                        &mut updateHandle,
                    );
                    match error != 0 {
                        true => {
                            ::log::info!("ota begin error begining, err == {:?}", error);
                            // abort current update
                            esp_idf_sys::esp_ota_abort(updateHandle);

                            // error so nak
                            sender
                                .send(BluetoothOperation::OTAControlRequestNack)
                                .unwrap();
                        }
                        false => {
                            // set variables with update information
                            ::log::info!("reading in packet size");

                            ota_logic_control_char_thread.lock().updating = true;
                            match value.len() {
                                1 => {
                                    ota_logic_write_thread.lock().packet_size =
                                        *value.index(0) as u16;
                                }
                                2.. => {
                                    ota_logic_write_thread.lock().packet_size =
                                        ((*value.index(1) as u16) << 8) + *value.index(0) as u16;
                                }
                                _ => {}
                            }

                            ota_logic_write_thread.lock().packets_received = 0;

                            ::log::info!("control request ack");

                            // no error so ack
                            sender
                                .send(BluetoothOperation::OTAControlRequestAck)
                                .unwrap();
                        }
                    }
                }
            }
            OTA_CONTROL_DONE => {
                ota_logic_write_thread.lock().updating = false; // we are no longer updating
                                                                // end the ota
                let err = unsafe { esp_ota_end(updateHandle) };
                match err == 0 {
                    true => {
                        sender.send(BluetoothOperation::OTAControldoneAck).unwrap();
                    }
                    false => {
                        if err == esp_idf_sys::ESP_ERR_OTA_VALIDATE_FAILED {
                            ::log::info!("Image validation failed, image is corrupted!")
                        } else {
                            ::log::info!("esp ota end failed, err = {:?}", err)
                        }
                        sender.send(BluetoothOperation::OTAControldoneNak).unwrap();
                    }
                }
            }
            _ => {
                // default do nothing
            }
        };
    });

    let ota_data_characteristic = ota_service.lock().create_characteristic(
        uuid128!("23408888-1F40-4CD8-9B89-CA8D45F8A5B0"),
        NimbleProperties::READ | NimbleProperties::WRITE,
    );

    ota_data_characteristic
        .lock()
        .on_write(move |data, _connection| {
            if ota_logic_data_thread.lock().updating {
                let err = unsafe {
                    esp_ota_write(
                        updateHandle,
                        data.as_ptr() as *const _,
                        data.len(), // yolo
                    )
                };

                if err != 0 {
                    //n
                    ::log::info!("data: {:?}, len: {:?}", data, data.len());
                    ::log::info!("esp ota write failed: err == {:?}", err);
                }

                // increment packets received
                ota_logic_data_thread.lock().packets_received += 1;
            }
        });

    let ota_apply_update_characteristic = ota_service.lock().create_characteristic(
        uuid128!("3e33db7b-9108-4549-b063-979f55610f0f"),
        NimbleProperties::READ | NimbleProperties::WRITE | NimbleProperties::NOTIFY,
    );

    /*
        this applies the update if any connected device writes to the char. should change to have a key to apply the update
    */
    ota_apply_update_characteristic
        .lock()
        .on_write(move |_, _connection| {
            match ota_logic_apply_update_thread.lock().update_available {
                true => {
                    // try to set the boot partition, match on error
                    let mut error = 0;
                    unsafe {
                        let partition = esp_idf_sys::esp_ota_get_next_update_partition(ptr::null());
                        error = esp_ota_set_boot_partition(partition);
                    }

                    // update fail if cannot set new partition
                    match error == 0 {
                        true => {
                            sender3
                                .send(BluetoothOperation::ApplyUpdateUpdateSuccess)
                                .unwrap();
                        }
                        false => {
                            sender3
                                .send(BluetoothOperation::ApplyUpdateUpdateFail)
                                .unwrap();
                        }
                    }
                }
                false => {
                    sender2
                        .send(BluetoothOperation::ApplyUpdateNoneAvailable)
                        .unwrap();
                }
            }
        });

    // start advertising
    let ble_advertising = ble_device.get_advertising();
    ble_advertising.name("esp32");
    ble_advertising.start().unwrap();

    loop {
        match receiver.recv().unwrap() {
            /* Apply update mechanisim */
            BluetoothOperation::ApplyUpdateNoneAvailable => {
                ::log::info!("No Update Available!");
                ota_apply_update_characteristic
                    .lock()
                    .set_value(&(NONE_ATTEMPTED).to_be_bytes())
                    .notify();
            }
            BluetoothOperation::ApplyUpdateUpdateFail => {
                ::log::info!("Apply update failed!");
                ota_apply_update_characteristic
                    .lock()
                    .set_value(&(UPDATE_FAIL).to_be_bytes())
                    .notify();
            }
            BluetoothOperation::ApplyUpdateUpdateSuccess => {
                ::log::info!("apply update success!");
                ota_apply_update_characteristic
                    .lock()
                    .set_value(&(UPDATE_SUCCESS).to_be_bytes())
                    .notify();

                esp_idf_hal::delay::FreeRtos::delay_ms(5);
                unsafe { esp_idf_sys::esp_restart() }; // restart the device after update has been applied successfully
            }
            BluetoothOperation::OTAControlRequestNack => {
                ::log::info!("OTAControlRequestNack");
                ota_control_characteristic
                    .lock()
                    .set_value(&(OTA_CONTROL_REQUEST_NAK).to_be_bytes())
                    .notify();
            }
            BluetoothOperation::OTAControlRequestAck => {
                ::log::info!("OTAControlRequestAck");
                ota_control_characteristic
                    .lock()
                    .set_value(&(OTA_CONTROL_REQUEST_ACK).to_be_bytes())
                    .notify();
            }
            BluetoothOperation::OTAControldoneAck => {
                ::log::info!("OTAControldoneAck");
                ota_control_characteristic
                    .lock()
                    .set_value(&(OTA_CONTROL_DONE_ACK).to_be_bytes())
                    .notify();
            }
            BluetoothOperation::OTAControldoneNak => {
                ::log::info!("OTAControldoneNak");
                ota_control_characteristic
                    .lock()
                    .set_value(&(OTA_CONTROL_DONE_NAK).to_be_bytes())
                    .notify();
            }
        }

        // sleep for 120
        esp_idf_hal::delay::FreeRtos::delay_ms(1000);
    }
}

// ota logic contains the data required for the ota update app logic
struct OtaLogic {
    updating: bool,
    update_available: bool,
    packet_size: u16,
    packets_received: u16,
}

// Application operations that can be queued
enum BluetoothOperation {
    ApplyUpdateNoneAvailable,
    ApplyUpdateUpdateFail,
    ApplyUpdateUpdateSuccess,
    OTAControlRequestNack,
    OTAControlRequestAck,
    OTAControldoneAck,
    OTAControldoneNak,
}

// /* messaging service */
// let messaging_service = server.create_service(uuid128!("6a7e2945-d40e-4891-bdc4-b22494ee0539"));

// let messaging_control = messaging_service.lock().create_characteristic(
//     uuid128!("23464575-3164-4dcb-b200-602ebd7cd3f0"),
//     NimbleProperties::READ | NimbleProperties::WRITE | NimbleProperties::NOTIFY,
// );

// let messaging_data = messaging_service.lock().create_characteristic(
//     uuid128!("a241328d-fd06-4475-a31a-26328d92eba2"),
//     NimbleProperties::READ | NimbleProperties::WRITE | NimbleProperties::NOTIFY,
// );
