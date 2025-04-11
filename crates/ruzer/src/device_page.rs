use gtk::prelude::*;
use nusb::DeviceInfo;
use relm4::prelude::*;

pub struct DevicePage {
    pub device_info: Option<DeviceInfo>,
}

#[derive(Debug)]
pub enum DevicePageMsg {
    SelectDevice(DeviceInfo),
}

#[relm4::component(pub)]
impl Component for DevicePage {
    type Init = ();
    type Input = DevicePageMsg;
    type Output = ();
    type CommandOutput = ();

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 10,
            set_margin_all: 10,
            #[name = "label"]
            gtk::Label {
                set_label: "Device Page",
                set_halign: gtk::Align::Start,
                set_margin_top: 10,
                set_margin_bottom: 10,
            },
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Self { device_info: None };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::Input,
        _sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            DevicePageMsg::SelectDevice(device_info) => {
                self.device_info = Some(device_info);
                if let Some(device_info) = &self.device_info {
                    widgets
                        .label
                        .set_label(&format!("{}", device_info.product_string().unwrap()));
                }
            }
        }
    }
}
