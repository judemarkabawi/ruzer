use adw::prelude::*;
use device_list::{DeviceListing, DeviceListingOutput};
use device_page::{DevicePage, DevicePageMsg};
use driver::common::RAZER_USB_VENDOR_ID;
use nusb::DeviceInfo;
use relm4::prelude::*;

mod device_list;
mod device_page;

struct App {
    device_page: relm4::Controller<DevicePage>,
    device_list: FactoryVecDeque<DeviceListing>,
}

#[derive(Debug)]
enum AppPage {
    Home,
    Device(DeviceInfo),
}

#[derive(Debug)]
enum AppMsg {
    SwitchPage(AppPage),
    UpdateDeviceList,
}

fn scan_devices() -> Vec<DeviceInfo> {
    nusb::list_devices()
        .into_iter()
        .flatten()
        .filter(|device_info| device_info.vendor_id() == RAZER_USB_VENDOR_ID)
        .collect()
}

#[relm4::component]
impl Component for App {
    type CommandOutput = ();
    type Input = AppMsg;
    type Output = ();
    type Init = ();

    /// Initialize the UI and model.
    fn init(
        _: Self::Init,
        window: Self::Root,
        sender: ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let device_list = FactoryVecDeque::builder()
            .launch(
                gtk::ListBox::builder()
                    .selection_mode(gtk::SelectionMode::None)
                    .css_classes(["boxed-list"])
                    .valign(gtk::Align::Start)
                    .build(),
            )
            .forward(
                sender.input_sender(),
                |DeviceListingOutput::SelectDevice(device_info)| {
                    AppMsg::SwitchPage(AppPage::Device(device_info))
                },
            );

        let model = App {
            device_page: DevicePage::builder().launch(()).detach(),
            device_list,
        };
        sender.input(AppMsg::SwitchPage(AppPage::Home));

        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::Input,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            AppMsg::SwitchPage(AppPage::Device(device_info)) => {
                self.device_page
                    .sender()
                    .send(DevicePageMsg::SelectDevice(device_info))
                    .unwrap();
                widgets.root_stack.set_visible_child_name("device");
            }
            AppMsg::SwitchPage(AppPage::Home) => {
                sender.input(AppMsg::UpdateDeviceList);
                widgets.root_stack.set_visible_child_name("home");
            }
            AppMsg::UpdateDeviceList => {
                println!("Updating device list...");
                let mut device_list = self.device_list.guard();
                let devices_info = scan_devices();
                for device in devices_info {
                    println!("added device");
                    device_list.push_back(device);
                }
            }
        }
    }

    view! {
        adw::ApplicationWindow {
            set_title: Some("Ruzer"),
            set_default_width: 500,
            set_default_height: 500,

            adw::ToolbarView {
                #[name = "top_bar"]
                add_top_bar = &adw::HeaderBar {},

                #[wrap(Some)]
                #[name = "root_stack"]
                set_content = &gtk::Stack {
                    set_transition_type: gtk::StackTransitionType::Crossfade,
                    add_named[Some("home")] = &adw::Clamp {
                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            gtk::Label {
                                set_label: "Select a Device",
                                set_valign: gtk::Align::Start,
                                set_css_classes: &["title-1"]
                            },
                            model.device_list.widget(),
                        }
                    },
                    add_named[Some("device")] = model.device_page.widget(),
                },
            },

        }
    }
}

fn main() {
    let app = RelmApp::new("relm4.test.simple_manual");
    app.run::<App>(());
}
