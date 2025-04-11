use clap::{Args, Parser, Subcommand};
use driver::{
    chroma::ExtendedMatrixEffect,
    common::{NormalPollingRate, RAZER_USB_VENDOR_ID},
    devices::{RazerDevice, RazerDeviceClaimed},
};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Dpi(DpiCommand),
    Info,
    Led(LedCommand),
    PollingRate(PollingRateCommand),
}

#[derive(Parser, Debug)]
struct DpiCommand {
    #[command(subcommand)]
    command: Option<DpiAction>,
}

#[derive(Subcommand, Debug)]
enum DpiAction {
    Get,
    Set { dpi: u16 }, // TODO: Support x/y DPI separately
    GetStages,
    SetStages { dpis: Vec<u16> },
}

#[derive(Args, Debug)]
struct LedCommand {
    #[arg(short, long)]
    led: Option<Led>,
    #[command(subcommand)]
    effect: LedEffect,
}

#[derive(Subcommand, Clone, Debug)]
enum LedEffect {
    Off,
    Static {
        #[arg(short, long)]
        color: String,
    },
    #[command(subcommand)]
    Breathing(BreathingEffect),
    Spectrum,
    Reactive {
        #[arg(short, long)]
        color: String,
        #[arg(short, long)]
        speed: u8,
    },
}

#[derive(Subcommand, Clone, Debug)]
enum BreathingEffect {
    Random,
    Single { color: String },
    Dual { color1: String, color2: String },
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum Led {
    Logo,
}

#[derive(Parser, Debug)]
struct PollingRateCommand {
    #[command(subcommand)]
    command: Option<PollingRateAction>,
}

#[derive(Subcommand, Debug)]
enum PollingRateAction {
    Get,
    Set { value: u16 },
}

#[tokio::main]
async fn main() {
    let args = Cli::parse();

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
        Command::Dpi(command) => handle_dpi_command(&mouse, command).await,
        Command::Info => handle_info_command(&mouse).await,
        Command::Led(command) => handle_led_command(&mouse, command).await,
        Command::PollingRate(command) => handle_polling_rate_command(mouse, command).await,
    }
}

async fn handle_led_command(mouse: &RazerDeviceClaimed, command: LedCommand) {
    match command.effect {
        LedEffect::Off => {
            let result = mouse
                .chroma_logo_matrix_effect(ExtendedMatrixEffect::None)
                .await;
            if let Err(err) = result {
                println!("{}", err);
            }
        }
        LedEffect::Static { color } => {
            let color = color.parse();
            match color {
                Ok(color) => {
                    let result = mouse
                        .chroma_logo_matrix_effect(ExtendedMatrixEffect::Static(color))
                        .await;
                    if let Err(err) = result {
                        println!("{}", err);
                    }
                }
                Err(_) => {
                    println!("{}", color_err_msg());
                }
            }
        }
        LedEffect::Breathing(breathing_effect) => {
            let effect = match breathing_effect {
                BreathingEffect::Random => {
                    ExtendedMatrixEffect::Breathing(driver::chroma::BreathingEffect::Random)
                }
                BreathingEffect::Single { color } => {
                    let color = color.parse();
                    match color {
                        Ok(color) => ExtendedMatrixEffect::Breathing(
                            driver::chroma::BreathingEffect::Single(color),
                        ),
                        Err(_) => {
                            println!("{}", color_err_msg());
                            return;
                        }
                    }
                }
                BreathingEffect::Dual { color1, color2 } => {
                    let color1 = color1.parse();
                    let color2 = color2.parse();
                    match (color1, color2) {
                        (Ok(color1), Ok(color2)) => ExtendedMatrixEffect::Breathing(
                            driver::chroma::BreathingEffect::Dual(color1, color2),
                        ),
                        _ => {
                            println!("{}", color_err_msg());
                            return;
                        }
                    }
                }
            };
            let result = mouse.chroma_logo_matrix_effect(effect).await;
            if let Err(err) = result {
                println!("{}", err);
            }
        }
        LedEffect::Spectrum => {
            let result = mouse
                .chroma_logo_matrix_effect(ExtendedMatrixEffect::Spectrum)
                .await;
            if let Err(err) = result {
                println!("{}", err);
            }
        }
        LedEffect::Reactive { color, speed } => {
            let color = color.parse();
            match color {
                Ok(color) => {
                    let result = mouse
                        .chroma_logo_matrix_effect(ExtendedMatrixEffect::Reactive(color, speed))
                        .await;
                    if let Err(err) = result {
                        println!("{}", err);
                    }
                }
                Err(_) => {
                    println!("{}", color_err_msg())
                }
            }
        }
    }
}

async fn handle_dpi_command(mouse: &RazerDeviceClaimed, dpi_command: DpiCommand) {
    match dpi_command.command {
        Some(DpiAction::Get) | None => {
            let dpi = mouse
                .get_dpi()
                .await
                .map_or_else(|err| err.to_string(), |dpi| format!("DPI: {:?}", dpi));
            println!("{}", dpi);
        }
        Some(DpiAction::Set { dpi }) => {
            let result = mouse.set_dpi(dpi.into()).await;
            if let Err(err) = result {
                println!("{}", err);
            }
        }
        Some(DpiAction::GetStages) => {
            let dpi_stages = mouse.get_dpi_stages().await.map_or_else(
                |err| err.to_string(),
                |dpi_stages| format!("{:?}", dpi_stages),
            );
            println!("{}", dpi_stages);
        }
        Some(DpiAction::SetStages { dpis: _ }) => todo!(),
    }
}

async fn handle_info_command(mouse: &RazerDeviceClaimed) {
    let battery_level = mouse.get_battery_level().await.map_or_else(
        |err| err.to_string(),
        |battery_level| battery_level.to_string(),
    );
    println!("Battery Level: {}", battery_level);
}

async fn handle_polling_rate_command(mouse: RazerDeviceClaimed, command: PollingRateCommand) {
    match command.command {
        Some(PollingRateAction::Get) | None => {
            let polling_rate = mouse.get_polling_rate().await.map_or_else(
                |err| err.to_string(),
                |polling_rate| polling_rate.to_string(),
            );
            println!("Polling Rate: {}", polling_rate)
        }
        Some(PollingRateAction::Set { value }) => {
            // TODO: support extended polling rates
            let polling_rate = NormalPollingRate::try_from(value);
            match polling_rate {
                Ok(polling_rate) => {
                    let result = mouse.set_polling_rate(polling_rate.into()).await;
                    if let Err(err) = result {
                        println!("{}", err);
                    }
                }
                Err(_) => {
                    println!("Invalid polling rate. Must be one of: [125, 500, 1000]");
                }
            }
        }
    }
}

fn color_err_msg() -> &'static str {
    "Please specify a color in hex (ex: #0cff1d)"
}
