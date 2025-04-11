use driver::common::RAZER_USB_VENDOR_ID;
use iced::{
    widget::{button, column, container, pick_list, text},
    Element, Length, Task,
};
use nusb::{list_devices, DeviceInfo};

pub fn main() -> iced::Result {
    iced::application("Stopwatch - Iced", App::update, App::view)
        .theme(|_| iced::Theme::Dark)
        .run_with(App::init)
}

#[derive(Default)]
struct App {
    devices: Vec<DeviceInfo>,
}

#[derive(Debug, Clone)]
enum Message {
    ScanDevices,
}

impl App {
    fn init() -> (App, Task<Message>) {
        (App::default(), Task::done(Message::ScanDevices))
    }

    fn scan_devices(&mut self) {
        self.devices = nusb::list_devices()
            .into_iter()
            .flatten()
            .filter(|device_info| device_info.vendor_id() == RAZER_USB_VENDOR_ID)
            .collect();
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::ScanDevices => self.scan_devices(),
        }
    }

    fn view(&self) -> Element<Message> {
        let devices = self
            .devices
            .iter()
            // may be a problem if we can't get product string without I/O when otherwise it would be a fine device
            .map(|device| device.product_string().into_iter())
            .flatten();

        let devices_text: Vec<Element<Message>> =
            devices.map(|device_str| text(device_str).into()).collect();

        container(column(devices_text).spacing(10))
            .center_x(Length::Fill)
            .into()
    }
}
