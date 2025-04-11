use crate::chroma::{BreathingEffect, ExtendedMatrixEffect, LedId};
use anyhow::{anyhow, Error, Result};
use nusb::{
    transfer::{ControlIn, ControlOut, ControlType, Recipient},
    Interface,
};
use std::{
    cmp::{max, min},
    time::Duration,
};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

pub const RAZER_USB_VENDOR_ID: u16 = 0x1532;
pub(crate) const RAZER_REPORT_SIZE: usize = size_of::<RazerMessage>();
pub(crate) const RAZER_REPORT_ARGUMENT_SIZE: usize = 80;
pub(crate) const RAZER_USB_INTERFACE_NUMBER: u8 = 0x00;
pub(crate) const RAZER_MOUSE_WAIT_TIME: Duration = Duration::from_millis(60);
pub(crate) const RAZER_MOUSE_MAX_DPI_STAGES: u8 = 5;

// linux/hid.h
pub(crate) const HID_REQ_GET_REPORT: u8 = 0x01;
pub(crate) const HID_REQ_SET_REPORT: u8 = 0x09;

#[derive(Immutable, KnownLayout, IntoBytes, FromBytes, Debug)]
#[repr(C)]
pub(crate) struct RazerMessage {
    status: u8,
    transaction_id: u8,
    remaining_packets: u16,
    protocol_type: u8,
    data_size: u8,
    command_class: u8,
    command_id: u8,
    arguments: [u8; 80],
    crc: u8,
    reserved: u8,
}

impl RazerMessage {
    pub(crate) fn arguments(&self) -> &[u8; 80] {
        &self.arguments
    }
}

#[derive(Clone, Debug)]
pub struct Dpi {
    x: u16,
    y: u16,
}

impl From<u16> for Dpi {
    fn from(value: u16) -> Self {
        Dpi { x: value, y: value }
    }
}

impl From<(u16, u16)> for Dpi {
    fn from(value: (u16, u16)) -> Self {
        Dpi {
            x: value.0,
            y: value.1,
        }
    }
}

#[derive(Clone, Debug)]
pub struct DpiStages {
    pub(crate) active: u8,
    pub(crate) stages: Vec<(u16, u16)>,
}

impl DpiStages {
    pub fn new(active: u8, stages: Vec<(u16, u16)>) -> Result<DpiStages> {
        if stages.is_empty() || stages.len() > RAZER_MOUSE_MAX_DPI_STAGES as usize {
            Err(anyhow!("DpiStages: Need 1 <= # of DPI stages <= 256"))
        } else if active < 1 || active > stages.len() as u8 {
            Err(anyhow!("DpiStages: Need 1 <= active stage <= # of stages"))
        } else {
            Ok(DpiStages { active, stages })
        }
    }
}

#[derive(Debug)]
pub(crate) struct RazerMessageBuilder {
    transaction_id: u8,
    data_size: u8,
    command_class: u8,
    command_id: u8,
    arguments: [u8; RAZER_REPORT_ARGUMENT_SIZE],
}

impl RazerMessageBuilder {
    pub(crate) fn build(self) -> RazerMessage {
        let mut result = RazerMessage {
            status: 0x00,
            transaction_id: self.transaction_id,
            remaining_packets: 0x0000,
            protocol_type: 0x00,
            data_size: self.data_size,
            command_class: self.command_class,
            command_id: self.command_id,
            arguments: self.arguments,
            crc: 0x00,
            reserved: 0x00,
        };
        result.crc = RazerMessageBuilder::calculate_crc(&result);
        result
    }

    pub(crate) fn with_transaction_id(mut self, transaction_id: u8) -> Self {
        self.transaction_id = transaction_id;
        self
    }

    /// Message to send to the device asking for battery level.
    pub(crate) fn get_battery_level() -> Self {
        Self {
            data_size: 0x02,
            command_class: 0x07,
            command_id: 0x80,
            ..Default::default()
        }
    }

    pub(crate) fn get_charging_status() -> Self {
        Self {
            data_size: 0x02,
            command_class: 0x07,
            command_id: 0x84,
            ..Default::default()
        }
    }

    pub(crate) fn get_dpi(var_store: VarStoreId) -> Self {
        let mut msg = Self {
            data_size: 0x07,
            command_class: 0x04,
            command_id: 0x85,
            ..Default::default()
        };
        msg.arguments[0] = var_store as u8;
        msg
    }

    pub(crate) fn set_dpi(var_store: VarStoreId, dpi: Dpi) -> Self {
        let mut msg = Self {
            data_size: 0x07,
            command_class: 0x04,
            command_id: 0x05,
            ..Default::default()
        };
        let dpi_x = clamp(dpi.x, 100, 35000);
        let dpi_y = clamp(dpi.y, 100, 35000);

        msg.arguments[0] = var_store as u8;
        msg.arguments[1] = ((dpi_x >> 8) & 0x00FF) as u8;
        msg.arguments[2] = (dpi_x & 0xFF) as u8;
        msg.arguments[3] = ((dpi_y >> 8) & 0x00FF) as u8;
        msg.arguments[4] = (dpi_y & 0xFF) as u8;
        msg.arguments[5] = 0x00;
        msg.arguments[6] = 0x00;
        msg
    }

    pub(crate) fn get_dpi_stages(var_store: VarStoreId) -> Self {
        let mut msg = Self {
            data_size: 0x26,
            command_class: 0x04,
            command_id: 0x86,
            ..Default::default()
        };
        msg.arguments[0] = var_store as u8;
        msg
    }

    pub(crate) fn set_dpi_stages(var_store: VarStoreId, dpi_stages: &DpiStages) -> Self {
        let mut msg = Self {
            data_size: 0x26,
            command_class: 0x04,
            command_id: 0x06,
            ..Default::default()
        };

        msg.arguments[0] = var_store as u8;
        msg.arguments[1] = dpi_stages.active;

        let num_stages = dpi_stages.stages.len();
        msg.arguments[2] = num_stages as u8;

        // We write for each stage
        // nn       - stage number
        // xx xx    - DPI X (u16)
        // yy yy    - DPI Y (u16)
        // 00 00    - Reserved
        msg.arguments[3..]
            .chunks_exact_mut(0x07)
            .take(num_stages)
            .enumerate()
            .for_each(|(i, chunk)| {
                let (dpi_x, dpi_y) = dpi_stages.stages[i];
                chunk[0] = i as u8;
                chunk[1..=2].copy_from_slice(&encode_u16_as_bytes(dpi_x));
                chunk[3..=4].copy_from_slice(&encode_u16_as_bytes(dpi_y));
                chunk[5] = 0;
                chunk[6] = 0;
            });
        msg
    }

    pub(crate) fn get_polling_rate() -> Self {
        Self {
            data_size: 0x01,
            command_class: 0x00,
            command_id: 0x85,
            ..Default::default()
        }
    }

    pub(crate) fn set_polling_rate(polling_rate: NormalPollingRate) -> Self {
        let mut msg = Self {
            data_size: 0x01,
            command_class: 0x00,
            command_id: 0x05,
            ..Default::default()
        };
        msg.arguments[0] = match polling_rate {
            NormalPollingRate::Rate1000 => 0x01,
            NormalPollingRate::Rate500 => 0x02,
            NormalPollingRate::Rate125 => 0x08,
        };
        msg
    }

    #[allow(unused)]
    pub(crate) fn set_polling_rate_extended(polling_rate: ExtendedPollingRate) -> Self {
        let mut msg = Self {
            data_size: 0x02,
            command_class: 0x00,
            command_id: 0x40,
            ..Default::default()
        };
        msg.arguments[0] = 0x00; // https://github.com/openrazer/openrazer/blob/16d13ef025d7cd32f03a6acc0548f9316e42b35a/driver/razermouse_driver.c#L1681
        msg.arguments[1] = match polling_rate {
            ExtendedPollingRate::Rate8000 => 0x01,
            ExtendedPollingRate::Rate4000 => 0x02,
            ExtendedPollingRate::Rate2000 => 0x04,
            ExtendedPollingRate::Rate1000 => 0x08,
            ExtendedPollingRate::Rate500 => 0x10,
            ExtendedPollingRate::Rate250 => 0x20,
            ExtendedPollingRate::Rate125 => 0x40,
        };
        msg
    }

    pub(crate) fn chroma_extended_matrix_effect(
        var_store: VarStoreId,
        led_id: LedId,
        effect: ExtendedMatrixEffect,
    ) -> Self {
        let mut msg = Self {
            command_class: 0x0F,
            command_id: 0x02,
            ..Default::default()
        };
        msg.arguments[0] = var_store as u8;
        msg.arguments[1] = led_id as u8;
        msg.arguments[2] = effect.into();

        match effect {
            ExtendedMatrixEffect::None | ExtendedMatrixEffect::Spectrum => {
                msg.data_size = 0x06;
            }
            ExtendedMatrixEffect::Static(color) => {
                let payload = [0x01, color.r, color.g, color.b];
                msg.arguments[5..=8].copy_from_slice(&payload);
                msg.data_size = 0x09;
            }
            ExtendedMatrixEffect::Breathing(effect) => match effect {
                BreathingEffect::Single(color) => {
                    let payload = [0x01, 0x00, 0x01, color.r, color.g, color.b];
                    msg.arguments[3..=8].copy_from_slice(&payload);
                    msg.data_size = 0x09;
                }
                BreathingEffect::Dual(color, color1) => {
                    let payload = [
                        0x02, 0x00, 0x02, color.r, color.g, color.b, color1.r, color1.g, color1.b,
                    ];
                    msg.arguments[3..=11].copy_from_slice(&payload);
                    msg.data_size = 0x0C;
                }
                BreathingEffect::Random => {
                    msg.data_size = 0x06;
                }
            },
            ExtendedMatrixEffect::Reactive(color, speed) => {
                let speed = clamp(speed, 0x01, 0x04);

                let payload = [speed, 0x01, color.r, color.g, color.b];
                msg.arguments[4..=8].copy_from_slice(&payload);
                msg.data_size = 0x09;
            }
        }
        msg
    }

    fn calculate_crc(report: &RazerMessage) -> u8 {
        let report = report.as_bytes();
        let mut crc: u8 = 0;
        // All the report except crc and reserved bytes
        for byte in report.iter().take(RAZER_REPORT_SIZE - 2).skip(2) {
            crc ^= byte;
        }
        crc
    }
}

impl Default for RazerMessageBuilder {
    fn default() -> Self {
        Self {
            transaction_id: 0,
            data_size: 0,
            command_class: 0,
            command_id: 0,
            arguments: [0; RAZER_REPORT_ARGUMENT_SIZE],
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum PollingRate {
    Normal(NormalPollingRate),
    Extended(ExtendedPollingRate),
}

impl From<PollingRate> for u16 {
    fn from(value: PollingRate) -> Self {
        match value {
            PollingRate::Normal(normal_polling_rate) => normal_polling_rate.into(),
            PollingRate::Extended(extended_polling_rate) => extended_polling_rate.into(),
        }
    }
}

impl From<NormalPollingRate> for PollingRate {
    fn from(value: NormalPollingRate) -> Self {
        Self::Normal(value)
    }
}

impl From<ExtendedPollingRate> for PollingRate {
    fn from(value: ExtendedPollingRate) -> Self {
        Self::Extended(value)
    }
}

impl std::fmt::Display for PollingRate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(u16::from(*self).to_string().as_ref())
    }
}

#[derive(Copy, Clone, Debug)]
pub enum NormalPollingRate {
    Rate1000,
    Rate500,
    Rate125,
}

impl From<NormalPollingRate> for u16 {
    fn from(value: NormalPollingRate) -> Self {
        match value {
            NormalPollingRate::Rate1000 => 1000,
            NormalPollingRate::Rate500 => 500,
            NormalPollingRate::Rate125 => 125,
        }
    }
}

impl TryFrom<u16> for NormalPollingRate {
    type Error = ();

    fn try_from(value: u16) -> std::result::Result<Self, Self::Error> {
        match value {
            1000 => Ok(NormalPollingRate::Rate1000),
            500 => Ok(NormalPollingRate::Rate500),
            125 => Ok(NormalPollingRate::Rate125),
            _ => Err(()),
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum ExtendedPollingRate {
    Rate8000,
    Rate4000,
    Rate2000,
    Rate1000,
    Rate500,
    Rate250,
    Rate125,
}

impl From<ExtendedPollingRate> for u16 {
    fn from(value: ExtendedPollingRate) -> Self {
        match value {
            ExtendedPollingRate::Rate8000 => 8000,
            ExtendedPollingRate::Rate4000 => 4000,
            ExtendedPollingRate::Rate2000 => 2000,
            ExtendedPollingRate::Rate1000 => 1000,
            ExtendedPollingRate::Rate500 => 500,
            ExtendedPollingRate::Rate250 => 250,
            ExtendedPollingRate::Rate125 => 125,
        }
    }
}

impl TryFrom<u16> for ExtendedPollingRate {
    type Error = ();

    fn try_from(value: u16) -> std::result::Result<Self, Self::Error> {
        match value {
            8000 => Ok(ExtendedPollingRate::Rate8000),
            4000 => Ok(ExtendedPollingRate::Rate4000),
            2000 => Ok(ExtendedPollingRate::Rate2000),
            1000 => Ok(ExtendedPollingRate::Rate1000),
            500 => Ok(ExtendedPollingRate::Rate500),
            250 => Ok(ExtendedPollingRate::Rate250),
            125 => Ok(ExtendedPollingRate::Rate125),
            _ => Err(()),
        }
    }
}

#[repr(u8)]
pub(crate) enum VarStoreId {
    NoStore = 0x00,
    VarStore = 0x01,
}

pub(crate) async fn send_razer_message(interface: Interface, request: RazerMessage) -> Result<()> {
    let control_message = usb_out_message(request.as_bytes());
    interface.control_out(control_message).await.into_result()?;
    Ok(())
}

pub(crate) async fn send_razer_message_and_wait_response(
    interface: Interface,
    request: RazerMessage,
) -> Result<RazerMessage> {
    send_razer_message(interface.clone(), request).await?;
    // Need to wait for some time before asking to avoid garbage response data
    tokio::time::sleep(RAZER_MOUSE_WAIT_TIME).await;

    // Get response
    let control_message = usb_in_message();
    let data = interface.control_in(control_message).await.into_result()?;
    let response = RazerMessage::read_from_bytes(&data)
        .map_err(|_| Error::msg("Invalid size of byte response"))?;
    Ok(response)
}

fn usb_out_message(data: &[u8]) -> ControlOut {
    ControlOut {
        control_type: ControlType::Class,
        recipient: Recipient::Interface,
        request: HID_REQ_SET_REPORT,
        value: 0x300,
        index: 0x00,
        data,
    }
}

fn usb_in_message() -> ControlIn {
    ControlIn {
        control_type: ControlType::Class,
        recipient: Recipient::Interface,
        request: HID_REQ_GET_REPORT,
        value: 0x300,
        index: 0x00,
        length: RAZER_REPORT_SIZE as u16,
    }
}

pub(crate) fn clamp<T: Ord>(val: T, min_range: T, max_range: T) -> T {
    min(max(min_range, val), max_range)
}

/// Big endian
pub(crate) fn decode_u16_from_bytes(val: &[u8]) -> u16 {
    ((val[0] as u16) << 8) | ((val[1] as u16) & 0xFF)
}

/// Big endian
pub(crate) fn encode_u16_as_bytes(val: u16) -> [u8; 2] {
    [((val >> 8) & 0xFF) as u8, (val & 0xFF) as u8]
}
