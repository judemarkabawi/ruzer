use adw::prelude::*;
use nusb::DeviceInfo;
use relm4::prelude::*;

#[derive(Debug)]
pub struct DeviceListing {
    device: DeviceInfo,
}

#[derive(Debug)]
pub enum DeviceListingOutput {
    SelectDevice(DeviceInfo),
}

#[relm4::factory(pub)]
impl FactoryComponent for DeviceListing {
    type ParentWidget = gtk::ListBox;
    type CommandOutput = ();
    type Input = ();
    type Output = DeviceListingOutput;
    type Init = DeviceInfo;

    fn init_model(init: Self::Init, _index: &Self::Index, _sender: FactorySender<Self>) -> Self {
        Self { device: init }
    }

    view! {
        adw::ActionRow {
            set_activatable: true,
            set_title: self.device.product_string().unwrap_or("Unknown Device").into(),
            connect_activated[sender, device = self.device.clone()] => move |_| {
                sender.output(DeviceListingOutput::SelectDevice(device.clone())).unwrap();
            }
        }
    }
}
