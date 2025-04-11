use adw::prelude::*;
use nusb::DeviceInfo;
use relm4::prelude::*;

use driver::common::RAZER_USB_VENDOR_ID;

fn scan_devices() -> Vec<DeviceInfo> {
    nusb::list_devices()
        .into_iter()
        .flatten()
        .filter(|device_info| device_info.vendor_id() == RAZER_USB_VENDOR_ID)
        .collect()
}

#[derive(Debug)]
pub struct DeviceListing {
    device: DeviceInfo,
}

#[derive(Debug)]
pub enum HomePageOutput {
    SelectDevice(DeviceInfo),
}

#[relm4::factory(pub)]
impl FactoryComponent for DeviceListing {
    type ParentWidget = gtk::ListBox;
    type CommandOutput = ();
    type Input = ();
    type Output = HomePageOutput;
    type Init = DeviceInfo;

    fn init_model(init: Self::Init, _index: &Self::Index, _sender: FactorySender<Self>) -> Self {
        Self { device: init }
    }

    view! {
        adw::ActionRow {
            set_activatable: true,
            set_title: self.device.product_string().unwrap_or("Unknown Device"),
            connect_activated[sender, device = self.device.clone()] => move |_| {
                sender.output(HomePageOutput::SelectDevice(device.clone())).unwrap();
            }
        }
    }
}

#[derive(Debug)]
pub struct HomePage {
    pub device_list: FactoryVecDeque<DeviceListing>,
}

#[derive(Debug)]
pub enum HomePageMsg {
    UpdateDeviceList,
}

#[relm4::component(pub)]
impl Component for HomePage {
    type CommandOutput = ();
    type Input = HomePageMsg;
    type Output = HomePageOutput;
    type Init = ();

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let device_list = FactoryVecDeque::builder()
            .launch(
                gtk::ListBox::builder()
                    .selection_mode(gtk::SelectionMode::None)
                    .css_classes(["boxed-list"])
                    .valign(gtk::Align::Start)
                    .build(),
            )
            .forward(sender.output_sender(), |msg| msg);

        let model = Self { device_list };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            HomePageMsg::UpdateDeviceList => {
                let mut device_list = self.device_list.guard();
                let devices_info = scan_devices();
                for device in devices_info {
                    device_list.push_back(device);
                }
            }
        }
    }

    view! {
        adw::Clamp {
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 10,
                set_margin_start: 20,
                set_margin_end: 20,
                gtk::Label {
                    set_label: "Select a Device",
                    set_valign: gtk::Align::Start,
                    set_css_classes: &["title-1"]
                },
                model.device_list.widget(),
            }
        }
    }
}
