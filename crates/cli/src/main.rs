use clap::{Parser, Subcommand};
use driver::{
    common::RAZER_USB_VENDOR_ID,
    devices::{RazerDevice, RazerDeviceClaimed},
};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Dpi(DpiCommand),
    Info,
    Led,
    PollingRate(PollingRateCommand),
}

#[derive(Parser, Debug)]
struct DpiCommand {
    #[command(subcommand)]
    command: Option<DpiAction>,
}

impl DpiCommand {
    fn command(&self) -> &DpiAction {
        match &self.command {
            Some(action) => action,
            None => &DpiAction::Get,
        }
    }
}

#[derive(Subcommand, Debug)]
enum DpiAction {
    Get,
    Set { dpi: u16 }, // TODO: Support x/y DPI separately
    GetStages,
    SetStages { dpis: Vec<u16> },
}

#[derive(Parser, Debug)]
struct PollingRateCommand {
    #[command(subcommand)]
    command: Option<PollingRateAction>,
}

#[derive(Subcommand, Debug)]
enum PollingRateAction {
    Get,
    // TODO: Set { value: u16 },
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let mouse_info = nusb::list_devices()
        .unwrap()
        .find(|device_info| device_info.vendor_id() == RAZER_USB_VENDOR_ID)
        .unwrap();
    let mouse = RazerDevice::new(mouse_info.clone()).claim().unwrap();

    let device_name = mouse_info.product_string().unwrap_or("Unknown device");
    println!("{}", device_name);

    handle_command(mouse, args.command).await;
}

async fn handle_command(mouse: RazerDeviceClaimed, command: Command) {
    match command {
        Command::Dpi(dpi_command) => handle_dpi(&mouse, dpi_command).await,
        Command::Info => handle_info(&mouse).await,
        Command::Led => todo!(),
        Command::PollingRate(command) => handle_polling_rate(mouse, command).await,
    }
}

async fn handle_dpi(mouse: &RazerDeviceClaimed, dpi_command: DpiCommand) {
    match dpi_command.command() {
        DpiAction::Get => {
            let dpi = mouse.get_dpi().await.map_or_else(
                |err| err.to_string(),
                |dpi| format!("DPI (x, y): ({}, {})", dpi.0, dpi.1),
            );
            println!("{}", dpi);
        }
        &DpiAction::Set { dpi } => {
            let result = mouse.set_dpi((dpi, dpi)).await;
            if let Err(err) = result {
                println!("{}", err);
            }
        }
        DpiAction::GetStages => {
            let dpi_stages = mouse.get_dpi_stages().await.map_or_else(
                |err| err.to_string(),
                |dpi_stages| format!("{:?}", dpi_stages),
            );
            println!("{}", dpi_stages);
        }
        DpiAction::SetStages { dpis: _ } => todo!(),
    }
}

async fn handle_info(mouse: &RazerDeviceClaimed) {
    let battery_level = mouse.get_battery_level().await.map_or_else(
        |err| err.to_string(),
        |battery_level| battery_level.to_string(),
    );
    println!("Battery Level: {}", battery_level);
}

async fn handle_polling_rate(mouse: RazerDeviceClaimed, command: PollingRateCommand) {
    match command.command {
        Some(PollingRateAction::Get) | None => {
            let polling_rate = mouse.get_polling_rate().await.map_or_else(
                |err| err.to_string(),
                |polling_rate| polling_rate.to_string(),
            );
            println!("Polling rate: {}", polling_rate)
        }
    }
}
