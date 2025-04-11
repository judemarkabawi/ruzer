use adw::prelude::*;
use driver::common::{Dpi, DpiStages};
use relm4::prelude::*;

#[derive(Clone, Debug)]
pub struct DpiStagesListing {
    is_active: bool,
    dpi: Dpi,
}

#[derive(Debug)]
pub enum DpiStagesListingOutput {
    Remove(DynamicIndex),
}

#[relm4::factory(pub)]
impl FactoryComponent for DpiStagesListing {
    type ParentWidget = gtk::ListBox;
    type CommandOutput = ();
    type Input = ();
    type Output = DpiStagesListingOutput;
    type Init = DpiStagesListing;

    fn init_model(init: Self::Init, _index: &Self::Index, _sender: FactorySender<Self>) -> Self {
        init
    }

    view! {
        adw::ActionRow {
            set_activatable: false,
            set_title: &self.dpi.x.to_string(),
            add_suffix = &gtk::Button {
                set_has_frame: false,
                set_valign: gtk::Align::Center,
                set_icon_name: "edit-delete-symbolic",
                connect_clicked[sender, index] => move |_| {
                    sender.output(DpiStagesListingOutput::Remove(index.clone())).unwrap();
                },
            },
        },
    }
}

#[derive(Debug)]
pub struct DpiStagesList {
    dpi_stages: FactoryVecDeque<DpiStagesListing>,
}

#[derive(Debug)]
pub enum DpiStagesListMsg {
    Add(Dpi),
    Update(DpiStages),
    Remove(DynamicIndex),
}

#[derive(Debug)]
pub enum DpiStagesListOutput {
    UpdatePending(DpiStages),
}

#[relm4::component(pub)]
impl Component for DpiStagesList {
    type CommandOutput = ();
    type Input = DpiStagesListMsg;
    type Output = DpiStagesListOutput;
    type Init = ();

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let dpi_stages = FactoryVecDeque::builder()
            .launch(gtk::ListBox::new())
            .forward(sender.input_sender(), |msg| match msg {
                DpiStagesListingOutput::Remove(index) => DpiStagesListMsg::Remove(index),
            });
        let model = DpiStagesList { dpi_stages };
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            DpiStagesListMsg::Add(dpi) => {
                // Add a new DPI stage
                {
                    let mut dpi_stages_list = self.dpi_stages.guard();
                    dpi_stages_list.push_back(DpiStagesListing {
                        is_active: false,
                        dpi,
                    });
                }
                self.sort_dpi_stages_list();
                self.notify_output_dpi_stages(sender);
            }
            DpiStagesListMsg::Remove(index) => {
                // Remove a DPI stage
                {
                    let mut dpi_stages_list = self.dpi_stages.guard();
                    dpi_stages_list.remove(index.current_index());
                }
                self.notify_output_dpi_stages(sender);
            }
            DpiStagesListMsg::Update(dpi_stages) => {
                let mut dpi_stages_list = self.dpi_stages.guard();
                dpi_stages_list.clear();
                for (index, &dpi) in dpi_stages.stages().iter().enumerate() {
                    dpi_stages_list.push_back(DpiStagesListing {
                        is_active: dpi_stages.active() as usize == index,
                        dpi,
                    });
                }
            }
        }
    }

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 20,
            gtk::Label {
                set_label: "DPI Stages",
                set_halign: gtk::Align::Start,
                set_css_classes: &["heading"],
            },
            model.dpi_stages.widget() -> &gtk::ListBox {
            },
            gtk::ListBox {
                set_selection_mode: gtk::SelectionMode::None,
                set_css_classes: &["boxed-list"],
                adw::EntryRow {
                    set_title: "Add DPI stage",
                    set_show_apply_button: true,
                    connect_apply[sender] => move |entry_row| {
                        let dpi = entry_row.text().parse::<u16>().ok()
                            .map(|dpi| dpi.into());

                        if let Some(dpi) = dpi {
                            sender.input(DpiStagesListMsg::Add(dpi))
                        } else {
                            entry_row.set_text("");
                        }
                    },
                },
            },
        },
    }
}

impl DpiStagesList {
    /// Send an output message to the rest of the app to update pending DPI stages changes
    fn notify_output_dpi_stages(&mut self, sender: ComponentSender<Self>) {
        let new_stages = self.get_dpi_stages_from_current();
        if let Ok(new_stages) = new_stages {
            let _ = sender.output(DpiStagesListOutput::UpdatePending(new_stages));
        }
    }

    /// Look at the current widgets and extract the DPI stages
    fn get_dpi_stages_from_current(&self) -> anyhow::Result<DpiStages> {
        let mut dpis: Vec<Dpi> = Vec::new();
        let mut active_stage = 0;
        for (index, listing) in self.dpi_stages.iter().enumerate() {
            if listing.is_active {
                active_stage = index;
            }
            dpis.push(listing.dpi);
        }

        DpiStages::new(active_stage as u8, dpis)
    }

    fn sort_dpi_stages_list(&mut self) {
        let mut dpi_stages_list = self.dpi_stages.guard();
        let mut new_list: Vec<_> = dpi_stages_list.iter().cloned().collect();
        new_list.sort_by(|a, b| a.dpi.x.cmp(&b.dpi.x));

        dpi_stages_list.clear();
        for dpi_stage_listing in new_list {
            dpi_stages_list.push_back(dpi_stage_listing);
        }
    }
}
