use std::{
    io::Write,
    ops::Index,
    sync::{
        self,
        mpsc::{sync_channel, Receiver, SyncSender},
        Arc,
    },
    u8,
};

use esp32_nimble::{
    utilities::mutex::RawMutex, uuid128, BLECharacteristic, BLEDevice, NimbleProperties,
};

use esp_idf_sys::{
    self as _, esp_ota_end, esp_ota_handle_t, esp_ota_set_boot_partition, esp_ota_write,
};

// let a = 0;
// global update handle
// let mut updateHandle = esp_ota_handle_t::from;
static mut UPDATE_HANDLE: esp_ota_handle_t = 0;

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

struct BLEOperationMessage {
    operation: BluetoothOperation,
    data: Vec<u8>,
}

impl BLEOperationMessage {
    fn set_and_notify(
        &self,
        char: Arc<embedded_svc::utils::mutex::Mutex<RawMutex, BLECharacteristic>>,
    ) {
        char.lock().set_value(&self.data).notify();
    }
}

pub struct BluetoothProcessing {
    ble_op_sender: SyncSender<BLEOperationMessage>,
    ble_op_receiver: Receiver<BLEOperationMessage>,
    ble_device: Option<Arc<sync::Mutex<&'static mut BLEDevice>>>,
    ota_logic: Arc<sync::Mutex<OtaLogic>>,
    byte_input_stream: Arc<sync::Mutex<Vec<u8>>>,
    byte_output_stream: Arc<sync::Mutex<Vec<u8>>>,
}

impl BluetoothProcessing {
    pub fn init_device(mut self) -> Self {
        let ble_device = BLEDevice::take();

        ble_device
            .set_power(
                esp32_nimble::enums::PowerType::Default,
                esp32_nimble::enums::PowerLevel::N0,
            )
            .unwrap();
        self.ble_device = Some(Arc::new(sync::Mutex::new(ble_device)));
        self
    }
    pub fn init_server(self) -> Self {
        let device = Arc::clone(self.ble_device.as_ref().unwrap());
        let device_arc_a = Arc::clone(self.ble_device.as_ref().unwrap()); // reference for arc
        let device_arc_b = Arc::clone(self.ble_device.as_ref().unwrap()); // reference for arc

        // self.device()
        device
            .lock()
            .unwrap()
            .set_power(
                esp32_nimble::enums::PowerType::Default,
                esp32_nimble::enums::PowerLevel::P3,
            )
            .unwrap();

        device.lock().unwrap().get_server().on_connect(move |_| {
            ::log::info!("Client connected");
            device_arc_a
                .lock()
                .unwrap()
                .get_advertising()
                .stop()
                .unwrap_or_else(|err| println!("error stopping advertising {:?}", err));
        });

        device.lock().unwrap().get_server().on_disconnect(move |_| {
            ::log::info!("start advertising again, client disconnected");

            device_arc_b
                .lock()
                .unwrap()
                .get_advertising()
                .start()
                .unwrap_or_else(|_| {
                    ::log::info!("error starting advertising");
                });
        });
        self
    }
    pub fn new(
        byte_input_stream: Arc<sync::Mutex<Vec<u8>>>,
        byte_output_stream: Arc<sync::Mutex<Vec<u8>>>,
    ) -> Self {
        // create ota logic
        let ota_logic = Arc::new(sync::Mutex::new(OtaLogic {
            update_available: false,
            updating: false,
            packet_size: 0,
            packets_received: 0,
        }));

        // create channels for Bluetooth operations
        let (sender, receiver) = sync_channel::<BLEOperationMessage>(5);

        BluetoothProcessing {
            byte_input_stream,
            ble_op_sender: sender,
            ble_op_receiver: receiver,
            ota_logic,
            ble_device: None,
            byte_output_stream,
        }
    }
    pub fn run_ble(self) {
        // create characteristics
        let device = Arc::clone(self.ble_device.as_ref().unwrap());

        // create ota service
        let ota_service = device
            .lock()
            .unwrap()
            .get_server()
            .create_service(uuid128!("d6f1d96d-594c-4c53-b1c6-244a1dfde6d8"));

        let ota_control_characteristic = ota_service.lock().create_characteristic(
            uuid128!("7ad671aa-21c0-46a4-b722-270e3ae3d830"),
            NimbleProperties::READ | NimbleProperties::WRITE | NimbleProperties::NOTIFY,
        );

        let ota_logic_control_arc = Arc::clone(&self.ota_logic);
        let ble_op_control_sender = self.ble_op_sender.clone();
        ota_control_characteristic.lock().on_write(move |value, _| {
            if value.is_empty() {
                return; // return early if char value is null
            }

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
                            &mut UPDATE_HANDLE,
                        );
                        match error != 0 {
                            true => {
                                ::log::info!("ota begin error begining, err == {:?}", error);
                                esp_idf_sys::esp_ota_abort(UPDATE_HANDLE);
                                ble_op_control_sender
                                    .send(BLEOperationMessage {
                                        operation: BluetoothOperation::OTAControlRequestNack,
                                        data: OTA_CONTROL_REQUEST_NAK.to_be_bytes().to_vec(),
                                    })
                                    .unwrap();
                            }

                            false => {
                                // set variables with update information
                                ::log::info!("reading in packet size");

                                ota_logic_control_arc.lock().unwrap().updating = true;
                                match value.len() {
                                    1 => {
                                        ota_logic_control_arc.lock().unwrap().packet_size =
                                            *value.index(0) as u16;
                                    }
                                    2.. => {
                                        ota_logic_control_arc.lock().unwrap().packet_size =
                                            ((*value.index(1) as u16) << 8)
                                                + *value.index(0) as u16;
                                    }
                                    _ => {}
                                }

                                ota_logic_control_arc.lock().unwrap().packets_received = 0;

                                ::log::info!("control request ack");

                                // no error so ack
                                ble_op_control_sender
                                    .send(BLEOperationMessage {
                                        operation: BluetoothOperation::OTAControlRequestAck,
                                        data: OTA_CONTROL_REQUEST_ACK.to_be_bytes().to_vec(),
                                    })
                                    .unwrap();
                            }
                        }
                    }
                }
                OTA_CONTROL_DONE => {
                    ota_logic_control_arc.lock().unwrap().updating = false; // we are no longer updating
                                                                            // end the ota
                    let err = unsafe { esp_ota_end(UPDATE_HANDLE) };
                    match err == 0 {
                        true => {
                            ota_logic_control_arc.lock().unwrap().update_available = true;
                            ble_op_control_sender
                                .send(BLEOperationMessage {
                                    operation: BluetoothOperation::OTAControldoneAck,
                                    data: OTA_CONTROL_DONE_ACK.to_be_bytes().to_vec(),
                                })
                                .unwrap();
                        }
                        false => {
                            if err == esp_idf_sys::ESP_ERR_OTA_VALIDATE_FAILED {
                                ::log::info!("Image validation failed, image is corrupted!")
                            } else {
                                ::log::info!("esp ota end failed, err = {:?}", err)
                            }
                            ble_op_control_sender
                                .send(BLEOperationMessage {
                                    operation: BluetoothOperation::OTAControldoneNak,
                                    data: OTA_CONTROL_DONE_NAK.to_be_bytes().to_vec(),
                                })
                                .unwrap();
                        }
                    }
                }
                _ => {}
            };
        });

        // ota data characteristic
        let ota_data_characteristic = ota_service.lock().create_characteristic(
            uuid128!("23408888-1F40-4CD8-9B89-CA8D45F8A5B0"),
            NimbleProperties::READ | NimbleProperties::WRITE,
        );

        // arcs and clones
        let ota_logic_data_arc = Arc::clone(&self.ota_logic);
        ota_data_characteristic
            .lock()
            .on_write(move |data, _connection| {
                if ota_logic_data_arc.lock().unwrap().updating {
                    let err = unsafe {
                        esp_ota_write(UPDATE_HANDLE, data.as_ptr() as *const _, data.len())
                    };
                    if err != 0 {
                        ::log::info!("data: {:?}, len: {:?}", data, data.len());
                        ::log::info!("esp ota write failed: err == {:?}", err);
                    }
                    ota_logic_data_arc.lock().unwrap().packets_received += 1;
                }
            });

        let ota_apply_update_characteristic = ota_service.lock().create_characteristic(
            uuid128!("3e33db7b-9108-4549-b063-979f55610f0f"),
            NimbleProperties::READ | NimbleProperties::WRITE | NimbleProperties::NOTIFY,
        );

        /*
            this applies the update if any connected device writes to the char. should change to have a key to apply the update
        */
        let ota_logic_apply_update_arc = Arc::clone(&self.ota_logic);
        let ble_op_apply_update_sender = self.ble_op_sender.clone();
        ota_apply_update_characteristic
            .lock()
            .on_write(move |_, _connection| {
                match ota_logic_apply_update_arc.lock().unwrap().update_available {
                    true => {
                        let partition = unsafe {
                            esp_idf_sys::esp_ota_get_next_update_partition(std::ptr::null())
                        };
                        let error = unsafe { esp_ota_set_boot_partition(partition) };
                        match error == 0 {
                            true => {
                                ble_op_apply_update_sender
                                    .send(BLEOperationMessage {
                                        operation: BluetoothOperation::ApplyUpdateUpdateSuccess,
                                        data: UPDATE_SUCCESS.to_be_bytes().to_vec(),
                                    })
                                    .unwrap();
                            }
                            false => {
                                ble_op_apply_update_sender
                                    .send(BLEOperationMessage {
                                        operation: BluetoothOperation::ApplyUpdateUpdateFail,
                                        data: UPDATE_FAIL.to_be_bytes().to_vec(),
                                    })
                                    .unwrap();
                            }
                        }
                    }
                    false => {
                        ble_op_apply_update_sender
                            .send(BLEOperationMessage {
                                operation: BluetoothOperation::ApplyUpdateNoneAvailable,
                                data: NONE_ATTEMPTED.to_ne_bytes().to_vec(),
                            })
                            .unwrap();
                    }
                }
            });

        /* messaging service */
        let data_stream_service = device
            .lock()
            .unwrap()
            .get_server()
            .create_service(uuid128!("6a7e2945-d40e-4891-bdc4-b22494ee0539"));

        // you can only write to this stream
        let byte_in_stream_characteristic = data_stream_service.lock().create_characteristic(
            uuid128!("23464575-3164-4dcb-b200-602ebd7cd3f0"),
            NimbleProperties::WRITE,
        );

        let byte_out_char = data_stream_service.lock().create_characteristic(
            uuid128!("a241328d-fd06-4475-a31a-26328d92eba2"),
            NimbleProperties::NOTIFY | NimbleProperties::READ,
        );

        // let byteWriter = Arc::clone(&self.byte_input_stream);
        byte_in_stream_characteristic
            .lock()
            .on_write(move |data, _conn| {
                println!("got data in ble processnig {:?}", data);
                let err = self.byte_input_stream.lock().unwrap().write(data);
                match err {
                    Ok(size) => {
                        println!("wrote {:?} bytes", size);
                    }
                    Err(_) => println!("error writing to byte inp stream: {:?}", err),
                }
            });

        // start advertising
        let ble_advertising = device.lock().unwrap().get_advertising();
        ble_advertising.name("esp32");
        ble_advertising.start().unwrap();

        loop {
            // send bytes if there are bytes to send
            let buffer_len = self.byte_output_stream.lock().unwrap().len();
            println!("capacity: {:?}", buffer_len);
            if buffer_len > 0 {
                println!("HERE");
                let mut temp_vec = Vec::<u8>::with_capacity(buffer_len);
                for val in
                    self.byte_output_stream
                        .lock()
                        .unwrap()
                        .drain(std::ops::RangeToInclusive {
                            end: buffer_len - 1,
                        })
                {
                    temp_vec.push(val);
                }

                println!("writing bytes: {:?}", temp_vec);
                byte_out_char.lock().set_value(&temp_vec).notify();
            }

            let recv_res = self
                .ble_op_receiver
                .recv_deadline(std::time::Instant::now());

            if let Ok(op) = recv_res {
                match op.operation {
                    BluetoothOperation::ApplyUpdateNoneAvailable
                    | BluetoothOperation::ApplyUpdateUpdateFail => {
                        op.set_and_notify(Arc::clone(&ota_apply_update_characteristic));
                    }
                    BluetoothOperation::ApplyUpdateUpdateSuccess => {
                        ::log::info!("apply update success!");
                        op.set_and_notify(Arc::clone(&ota_apply_update_characteristic));
                        esp_idf_hal::delay::FreeRtos::delay_ms(5);
                        unsafe { esp_idf_sys::esp_restart() }; // restart the device after update has been applied successfully
                    }
                    BluetoothOperation::OTAControlRequestNack
                    | BluetoothOperation::OTAControlRequestAck
                    | BluetoothOperation::OTAControldoneAck
                    | BluetoothOperation::OTAControldoneNak => {
                        ::log::info!("OTAControlRequestNack");
                        op.set_and_notify(Arc::clone(&ota_control_characteristic));
                    }
                }
            }
            esp_idf_hal::delay::FreeRtos::delay_ms(1000);
        }
    }
}
