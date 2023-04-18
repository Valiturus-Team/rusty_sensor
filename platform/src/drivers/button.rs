use crossbeam_channel::{unbounded, Receiver, Sender};

use app::domain::domain::{self, ButtonEvent};
use esp_idf_sys::{
    esp, gpio_config, gpio_config_t, gpio_install_isr_service, gpio_int_type_t_GPIO_INTR_NEGEDGE,
    gpio_isr_handler_add, gpio_mode_t_GPIO_MODE_INPUT, xPortGetTickRateHz, xQueueGenericCreate,
    xQueueGiveFromISR, xQueueReceive, QueueHandle_t, ESP_INTR_FLAG_IRAM,
};

static mut EVENT_QUEUE: Option<QueueHandle_t> = None;

const VALITURUS_BUTTON_PIN: i32 = 18;

#[link_section = ".iram0.text"]
unsafe extern "C" fn button_interrupt(_: *mut std::ffi::c_void) {
    xQueueGiveFromISR(EVENT_QUEUE.unwrap(), std::ptr::null_mut());
}

// Queue configurations
const QUEUE_TYPE_BASE: u8 = 0;
const ITEM_SIZE: u32 = 0; // we're not posting any actual data, just notifying
const QUEUE_SIZE: u32 = 1;

// button!
pub struct Button {
    sender: Sender<ButtonEvent>,
    receiver: Receiver<ButtonEvent>,
}

impl Button {
    pub fn new() -> Result<Self, esp_idf_sys::EspError> {
        const GPIO_NUM: i32 = 18;
        let io_conf = gpio_config_t {
            pin_bit_mask: 1 << VALITURUS_BUTTON_PIN,
            mode: gpio_mode_t_GPIO_MODE_INPUT,
            pull_up_en: true.into(),
            pull_down_en: false.into(),
            intr_type: gpio_int_type_t_GPIO_INTR_NEGEDGE, // positive edge trigger = button down
        };

        unsafe {
            // TODO -> add error handling if init fails

            // Writes the button configuration to the registers
            esp!(gpio_config(&io_conf));

            // Installs the generic GPIO interrupt handler
            esp!(gpio_install_isr_service(ESP_INTR_FLAG_IRAM as i32));

            // Instantiates the event queue
            EVENT_QUEUE = Some(xQueueGenericCreate(QUEUE_SIZE, ITEM_SIZE, QUEUE_TYPE_BASE));

            // Registers our function with the generic GPIO interrupt handler we installed earlier.
            let initRes = esp!(gpio_isr_handler_add(
                GPIO_NUM,
                Some(button_interrupt),
                std::ptr::null_mut()
            ));

            if initRes.is_err() {
                return Err(initRes.err().unwrap());
            }
        }
        let (sender, receiver) = unbounded::<domain::ButtonEvent>();

        return Ok(Button { sender, receiver });
    }

    // button_loop prints out button presses on the queue
    pub fn button_loop(self) {
        loop {
            unsafe {
                let wait_ticks: u32 = xPortGetTickRateHz() * 2; // wait 2 second
                let res = xQueueReceive(EVENT_QUEUE.unwrap(), std::ptr::null_mut(), wait_ticks);
                if res != 1 {
                    continue; // if we recevied anything but a one continue
                }
                ::log::info!("sending event!");
                // send button press event
                if let Err(err) = self.sender.send(true) {
                    ::log::info!("error sending button press event {:?}", err)
                }
            }
        }
    }
    pub fn button_channel(&self) -> Receiver<ButtonEvent> {
        self.receiver.clone()
    }
}
