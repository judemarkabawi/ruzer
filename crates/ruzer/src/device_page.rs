use adw::prelude::*;
use driver::batched::BatchedFeatureSet;
use nusb::DeviceInfo;
use relm4::prelude::*;

#[derive(Default)]
pub struct DevicePage {
    usb_device_info: Option<nusb::DeviceInfo>,
    device_name: Option<String>,
    razer_device_info: Option<driver::batched::DeviceInfo>,
}

#[derive(Debug)]
pub enum DevicePageMsg {
    PageUpdate(DeviceInfo),
}

#[derive(Debug)]
pub enum DevicePageCommand {
    Data(driver::batched::DeviceInfo),
}

#[relm4::component(pub)]
impl Component for DevicePage {
    type CommandOutput = DevicePageCommand;
    type Input = DevicePageMsg;
    type Output = ();
    type Init = ();

    fn init(
        _init: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Self::default();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            DevicePageMsg::PageUpdate(device_info) => {
                self.device_name = device_info
                    .product_string()
                    .map(|device_name| device_name.to_owned());
                self.usb_device_info = Some(device_info);

                // Run batched device info command on device if exists
                if let Some(device_info) = &self.usb_device_info {
                    let device_info = device_info.clone();
                    sender.oneshot_command(async move {
                        let device = driver::devices::RazerDevice::new(device_info);
                        let device_claimed = device.claim().unwrap();
                        DevicePageCommand::Data(device_claimed.get_batched().await)
                    });
                }
            }
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        _sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        let DevicePageCommand::Data(razer_device_info) = message;
        std::println!("{:#?}", razer_device_info);
        self.razer_device_info = Some(razer_device_info);
    }

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_halign: gtk::Align::Center,
            set_spacing: 10,
            set_margin_start: 20,
            set_margin_end: 20,
            gtk::Label {
                #[watch]
                set_label?: &model.device_name,
                set_css_classes: &["title-1"],
            },
            gtk::Box {
                set_homogeneous: true,
                gtk::Label {
                    set_halign: gtk::Align::Start,
                    #[watch]
                    set_label: &match model.razer_device_info.as_ref().map(|info| info.battery_level).flatten() {
                        Some(level) => format!("Battery: {:.0}%", level),
                        None => "Battery: N/A".into(),
                    },
                    set_css_classes: &["caption"]
                },
                gtk::Label {
                    set_halign: gtk::Align::End,
                    set_label: "Charging",
                    #[watch]
                    set_visible: model.razer_device_info.as_ref().map(|info| info.charging_status.unwrap_or(false)).unwrap_or(false),
                    set_css_classes: &["caption"]
                },
            },
            gtk::ListBox {
                set_selection_mode: gtk::SelectionMode::None,
                set_css_classes: &["boxed-list"],
                adw::ComboRow {
                    set_title: "Polling Rate",
                    #[wrap(Some)]
                    set_model = &gtk::StringList::new(&["125", "500", "1000"]),
                }
            }
        }
    }
}
