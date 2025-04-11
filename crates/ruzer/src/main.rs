use adw::prelude::*;
use device_page::{DevicePage, DevicePageMsg};
use home_page::{HomePage, HomePageMsg, HomePageOutput};
use nusb::DeviceInfo;
use relm4::prelude::*;

mod device_page;
mod home_page;

struct App {
    home_page: relm4::Controller<HomePage>,
    device_page: relm4::Controller<DevicePage>,
    current_page: AppPage,
}

#[derive(Debug)]
enum SwitchAppPage {
    Home,
    Device(DeviceInfo),
}

#[derive(Debug, PartialEq, Eq)]
enum AppPage {
    Home,
    Device,
}

#[derive(Debug)]
enum AppMsg {
    SwitchPage(SwitchAppPage),
    Refresh,
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
        let model = App {
            home_page: HomePage::builder().launch(()).forward(
                sender.input_sender(),
                |HomePageOutput::SelectDevice(device_info)| {
                    AppMsg::SwitchPage(SwitchAppPage::Device(device_info))
                },
            ),
            device_page: DevicePage::builder().launch(()).detach(),
            current_page: AppPage::Home,
        };
        sender.input(AppMsg::SwitchPage(SwitchAppPage::Home));

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
            AppMsg::SwitchPage(SwitchAppPage::Device(device_info)) => {
                self.current_page = AppPage::Device;
                self.device_page.emit(DevicePageMsg::Update(device_info));
                widgets.root_stack.set_visible_child_name("device");
            }
            AppMsg::SwitchPage(SwitchAppPage::Home) => {
                self.current_page = AppPage::Home;
                self.home_page.emit(HomePageMsg::UpdateDeviceList);
                widgets.root_stack.set_visible_child_name("home");
            }
            AppMsg::Refresh => {
                self.home_page.emit(HomePageMsg::UpdateDeviceList);
                self.device_page.emit(DevicePageMsg::Refresh);
            }
        }
    }

    view! {
        adw::ApplicationWindow {
            set_title: Some("Ruzer"),
            set_default_width: 1000,
            set_default_height: 750,

            adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {
                    #[name = "refresh_button"]
                    pack_end = &gtk::Button {
                        set_icon_name: "view-refresh-symbolic",
                        connect_clicked => AppMsg::Refresh,
                    },
                },

                #[wrap(Some)]
                #[name = "root_stack"]
                set_content = &gtk::Stack {
                    set_transition_type: gtk::StackTransitionType::Crossfade,
                    add_named[Some("home")] = model.home_page.widget(),
                    add_named[Some("device")] = model.device_page.widget(),
                },
            },
        }
    }
}

fn main() {
    let app = RelmApp::new("com.github.ruzer");
    app.run::<App>(());
}
