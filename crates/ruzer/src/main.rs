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
}

#[derive(Debug)]
enum AppPage {
    Home,
    Device(DeviceInfo),
}

#[derive(Debug)]
enum AppMsg {
    SwitchPage(AppPage),
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
                    AppMsg::SwitchPage(AppPage::Device(device_info))
                },
            ),
            device_page: DevicePage::builder().launch(()).detach(),
        };
        sender.input(AppMsg::SwitchPage(AppPage::Home));

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
            AppMsg::SwitchPage(AppPage::Device(device_info)) => {
                self.device_page
                    .sender()
                    .send(DevicePageMsg::Update(device_info))
                    .unwrap();
                widgets.root_stack.set_visible_child_name("device");
            }
            AppMsg::SwitchPage(AppPage::Home) => {
                self.home_page.emit(HomePageMsg::UpdateDeviceList);
                widgets.root_stack.set_visible_child_name("home");
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
