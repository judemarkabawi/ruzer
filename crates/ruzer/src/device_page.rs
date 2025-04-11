use adw::prelude::*;
use driver::{
    batched::{BatchedFeatureSet, DeviceSettings},
    common::NormalPollingRate,
};
use nusb::DeviceInfo;
use relm4::prelude::*;

mod dpi_stages;

pub struct DevicePage {
    usb_device_info: Option<nusb::DeviceInfo>,
    device_name: Option<String>,
    razer_device_info: driver::batched::DeviceInfo,
    dpi_stages_list: relm4::Controller<dpi_stages::DpiStagesList>,
    pending_changes: DeviceSettings,
}

#[derive(Debug)]
pub enum DevicePageMsg {
    Update(nusb::DeviceInfo),
    Refresh,
    SelectPollingRate(driver::common::PollingRate),
    SetDpi(Option<u16>),
    SetDpiStages(driver::common::DpiStages),
    Cancel,
    Apply,
}

#[derive(Debug)]
pub enum DevicePageCommand {
    Update(driver::batched::DeviceInfo),
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
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let dpi_stages_list =
            dpi_stages::DpiStagesList::builder()
                .launch(())
                .forward(sender.input_sender(), |msg| match msg {
                    dpi_stages::DpiStagesListOutput::UpdatePending(dpi_stages) => {
                        DevicePageMsg::SetDpiStages(dpi_stages)
                    }
                });
        let model = Self {
            usb_device_info: None,
            device_name: None,
            razer_device_info: driver::batched::DeviceInfo::default(),
            dpi_stages_list,
            pending_changes: DeviceSettings::default(),
        };
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            DevicePageMsg::Update(usb_device_info) => {
                self.update(&sender, usb_device_info);
            }
            DevicePageMsg::Refresh => {
                if let Some(usb_device_info) = self.usb_device_info.clone() {
                    self.update(&sender, usb_device_info);
                }
            }
            DevicePageMsg::SelectPollingRate(polling_rate) => {
                self.pending_changes.polling_rate = Some(polling_rate);
            }
            DevicePageMsg::SetDpi(dpi) => {
                let dpi_range = self.razer_device_info.dpi_range;
                let dpi = dpi
                    .filter(|dpi| dpi_range.0 <= *dpi && *dpi <= dpi_range.1)
                    .map(|dpi| dpi.into());
                self.pending_changes.dpi = dpi;
            }
            DevicePageMsg::Apply => {
                self.apply_changes(&sender);
            }
            DevicePageMsg::Cancel => {
                self.pending_changes = DeviceSettings::default();
                if let Some(dpi_stages) = self.razer_device_info.dpi_stages.clone() {
                    self.dpi_stages_list
                        .emit(dpi_stages::DpiStagesListMsg::Update(dpi_stages))
                }
            }
            DevicePageMsg::SetDpiStages(dpi_stages) => {
                self.pending_changes.dpi_stages = Some(dpi_stages);
            }
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        _sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            DevicePageCommand::Update(razer_device_info) => {
                // Reset page and update with new device info
                self.pending_changes = DeviceSettings::default();
                self.razer_device_info = razer_device_info.clone();
                if let Some(dpi_stages) = razer_device_info.dpi_stages {
                    self.dpi_stages_list
                        .emit(dpi_stages::DpiStagesListMsg::Update(dpi_stages))
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
                // Device and Battery Info Section
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_halign: gtk::Align::Center,
                    set_spacing: 5,
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
                            set_label: &match model.razer_device_info.battery_level {
                                Some(level) => format!("Battery: {:.0}%", level),
                                None => "Battery: N/A".into(),
                            },
                            set_css_classes: &["caption"]
                        },
                        gtk::Label {
                            set_halign: gtk::Align::End,
                            set_label: "Charging",
                            #[watch]
                            set_visible: model.razer_device_info.charging_status.unwrap_or(false),
                            set_css_classes: &["caption"]
                        },
                    },
                },
                // Controls Section
                gtk::Box {
                    set_spacing: 10,
                    set_orientation: gtk::Orientation::Vertical,
                    gtk::ListBox {
                        set_selection_mode: gtk::SelectionMode::None,
                        set_css_classes: &["boxed-list"],
                        // Polling Rate Section
                        adw::ComboRow {
                            // TODO: Handle extended polling rates
                            set_title: "Polling Rate",
                            #[watch]
                            set_selected: {
                                // In StringList model below
                                let rate_to_index = |rate| match rate {
                                    driver::common::PollingRate::Normal(NormalPollingRate::Rate125) => 0,
                                    driver::common::PollingRate::Normal(NormalPollingRate::Rate500) => 1,
                                    driver::common::PollingRate::Normal(NormalPollingRate::Rate1000) => 2,
                                    _ => gtk::INVALID_LIST_POSITION,
                                };
                                // Use current selected rate if set, otherwise use device info
                                if let Some(polling_rate) = model.pending_changes.polling_rate {
                                    rate_to_index(polling_rate)
                                } else if let Some(polling_rate) = model.razer_device_info.polling_rate {
                                    rate_to_index(polling_rate)
                                } else {
                                    gtk::INVALID_LIST_POSITION
                                }
                            },
                            #[wrap(Some)]
                            set_model = &gtk::StringList::new(&["125", "500", "1000"]),
                            connect_selected_notify[sender] => move |combo_row| {
                                let selected_string = combo_row
                                    .selected_item()
                                    .and_then(|obj| obj.downcast::<gtk::StringObject>().ok())
                                    .map(|s| Into::<String>::into(s.string()));
                                let polling_rate = selected_string
                                    .and_then(|s| s.parse::<u16>().ok())
                                    .and_then(|dpi| NormalPollingRate::try_from(dpi).ok());
                                if let Some(polling_rate) = polling_rate {
                                    sender.input(DevicePageMsg::SelectPollingRate(polling_rate.into()));
                                }
                            },
                        },
                        // DPI Section
                        adw::EntryRow {
                            set_title: "DPI",
                            set_show_apply_button: true,
                            #[watch]
                            set_text: &{
                                if let Some(dpi) = model.pending_changes.dpi {
                                    dpi.x.to_string()
                                } else if let Some(dpi) = model.razer_device_info.dpi {
                                    dpi.x.to_string()
                                } else {
                                    "".to_string()
                                }
                            },
                            connect_apply[sender] => move |entry_row| {
                                let dpi = entry_row.text().parse::<u16>().ok();
                                sender.input(DevicePageMsg::SetDpi(dpi));
                            },
                        },
                    },
                    model.dpi_stages_list.widget(),
                },
                // Apply Section
                gtk::Box {
                    set_spacing: 10,
                    set_halign: gtk::Align::End,
                    #[watch]
                    set_visible: settings_changed(&model.razer_device_info, &model.pending_changes),
                    gtk::Button {
                        set_label: "Apply",
                        set_css_classes: &["suggested-action"],
                        connect_clicked => DevicePageMsg::Apply,
                    },
                    gtk::Button {
                        set_label: "Cancel",
                        // set_css_classes: &["suggested-action"],
                        connect_clicked => DevicePageMsg::Cancel,
                    },
                }
            }
        }
    }
}

impl DevicePage {
    fn update(&mut self, sender: &ComponentSender<DevicePage>, usb_device_info: DeviceInfo) {
        self.device_name = usb_device_info
            .product_string()
            .map(|device_name| device_name.to_owned());
        self.usb_device_info = Some(usb_device_info.clone());

        // Run batched device info command on device if exists
        sender.oneshot_command(async move {
            let device = driver::devices::RazerDevice::new(usb_device_info);
            let device_claimed = device.claim().unwrap();
            DevicePageCommand::Update(device_claimed.get_batched().await)
        });
    }

    fn apply_changes(&self, sender: &ComponentSender<DevicePage>) {
        if let Some(device_info) = &self.usb_device_info {
            let device_info = device_info.clone();
            let pending_changes = self.pending_changes.clone();
            sender.oneshot_command(async move {
                let device = driver::devices::RazerDevice::new(device_info);
                let device_claimed = device.claim().unwrap();
                let _err = device_claimed.set_batched(&pending_changes).await;
                DevicePageCommand::Update(device_claimed.get_batched().await)
            });
        }
    }
}

fn settings_changed(info: &driver::batched::DeviceInfo, pending: &DeviceSettings) -> bool {
    (pending.dpi.is_some() && pending.dpi != info.dpi)
        || (pending.dpi_stages.is_some() && pending.dpi_stages != info.dpi_stages)
        || (pending.polling_rate.is_some() && pending.polling_rate != info.polling_rate)
}
