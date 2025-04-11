use std::ops::Deref;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use driver_macros::device_impls;
use nusb::{DeviceInfo, Interface};

use crate::{
    chroma::{ExtendedMatrixEffect, LedId},
    common::{
        decode_u16_from_bytes, send_razer_message, send_razer_message_and_wait_response, DpiStages,
        PollingRate, RazerMessageBuilder, VarStoreId, RAZER_USB_INTERFACE_NUMBER,
    },
};

#[async_trait]
pub trait FeatureSet: Send + Sync {
    async fn get_dpi(&self) -> Result<(u16, u16)> {
        Err(anyhow!("Unimplemented"))
    }
    async fn set_dpi(&self, _: (u16, u16)) -> Result<()> {
        Err(anyhow!("Unimplemented"))
    }
    async fn get_dpi_stages(&self) -> Result<DpiStages> {
        Err(anyhow!("Unimplemented"))
    }
    async fn set_dpi_stages(&self, _: &DpiStages) -> Result<()> {
        Err(anyhow!("Unimplemented"))
    }
    async fn get_polling_rate(&self) -> Result<u16> {
        Err(anyhow!("Unimplemented"))
    }
    async fn set_polling_rate(&self, _: PollingRate) -> Result<()> {
        Err(anyhow!("Unimplemented"))
    }
    async fn get_battery_level(&self) -> Result<f32> {
        Err(anyhow!("Unimplemented"))
    }
    async fn get_charging_status(&self) -> Result<bool> {
        Err(anyhow!("Unimplemented"))
    }
    async fn chroma_logo_matrix_effect(&self, _: ExtendedMatrixEffect) -> Result<()> {
        Err(anyhow!("Unimplemented"))
    }
}

pub struct RazerDevice(DeviceInfo);

impl RazerDevice {
    pub fn new(device_info: DeviceInfo) -> Self {
        RazerDevice(device_info)
    }

    pub fn claim(&self) -> Result<RazerDeviceClaimed> {
        let device = self.0.open()?;
        let interface = device.detach_and_claim_interface(RAZER_USB_INTERFACE_NUMBER)?;
        let device_impl = get_device_impl(self.0.product_id(), interface)?;
        Ok(RazerDeviceClaimed { device_impl })
    }
}

pub struct RazerDeviceClaimed {
    device_impl: Box<dyn FeatureSet>,
}

impl Deref for RazerDeviceClaimed {
    type Target = dyn FeatureSet;

    fn deref(&self) -> &Self::Target {
        &*self.device_impl
    }
}

async fn get_dpi(
    interface: Interface,
    transaction_id: u8,
    var_store: VarStoreId,
) -> Result<(u16, u16)> {
    let request = RazerMessageBuilder::get_dpi(var_store)
        .with_transaction_id(transaction_id)
        .build();
    let response = send_razer_message_and_wait_response(interface, request).await?;

    let dpi_x: u16 = decode_u16_from_bytes(&response.arguments()[1..=2]);
    let dpi_y: u16 = decode_u16_from_bytes(&response.arguments()[3..=4]);
    Ok((dpi_x, dpi_y))
}

async fn get_dpi_0x3f(interface: Interface) -> Result<(u16, u16)> {
    get_dpi(interface, 0x3f, VarStoreId::NoStore).await
}

async fn set_dpi(
    interface: Interface,
    dpi: (u16, u16),
    transaction_id: u8,
    var_store: VarStoreId,
) -> Result<()> {
    let request = RazerMessageBuilder::set_dpi(var_store, dpi)
        .with_transaction_id(transaction_id)
        .build();
    send_razer_message(interface, request).await
}

async fn set_dpi_0x3f(interface: Interface, dpi: (u16, u16)) -> Result<()> {
    set_dpi(interface, dpi, 0x3f, VarStoreId::NoStore).await
}

async fn get_dpi_stages(interface: Interface, transaction_id: u8) -> Result<DpiStages> {
    let request = RazerMessageBuilder::get_dpi_stages(VarStoreId::VarStore)
        .with_transaction_id(transaction_id)
        .build();
    let response = send_razer_message_and_wait_response(interface, request).await?;

    // Response format (hex):
    // 01    varstore
    // 02    active DPI stage (1 indexed)
    // 04    number of stages = 4
    //
    // 01    first DPI stage
    // 03 20 first stage DPI X = 800
    // 03 20 first stage DPI Y = 800
    // 00 00 reserved
    //
    // 02    second DPI stage
    // 07 08 second stage DPI X = 1800
    // 07 08 second stage DPI Y = 1800
    // 00 00 reserved
    //
    // 03    third DPI stage
    // ...
    let active_stage = response.arguments()[1];
    let num_stages = response.arguments()[2] as usize;
    let result = response.arguments()[3..]
        .chunks_exact(0x07)
        .take(num_stages)
        .map(|dpi_stage| {
            let dpi_x = decode_u16_from_bytes(&dpi_stage[1..=2]);
            let dpi_y = decode_u16_from_bytes(&dpi_stage[3..=4]);
            (dpi_x, dpi_y)
        })
        .collect();

    Ok(DpiStages {
        active: active_stage,
        stages: result,
    })
}

async fn get_dpi_stages_0x3f(interface: Interface) -> Result<DpiStages> {
    get_dpi_stages(interface, 0x3f).await
}

async fn set_dpi_stages(
    interface: Interface,
    dpi_stages: &DpiStages,
    transaction_id: u8,
) -> Result<()> {
    let request = RazerMessageBuilder::set_dpi_stages(VarStoreId::VarStore, dpi_stages)
        .with_transaction_id(transaction_id)
        .build();
    send_razer_message(interface, request).await
}

async fn set_dpi_stages_0x3f(interface: Interface, dpi_stages: &DpiStages) -> Result<()> {
    set_dpi_stages(interface, dpi_stages, 0x3f).await
}

async fn get_polling_rate(interface: Interface, transaction_id: u8) -> Result<u16> {
    let request = RazerMessageBuilder::get_polling_rate()
        .with_transaction_id(transaction_id)
        .build();
    let response = send_razer_message_and_wait_response(interface, request).await?;

    match response.arguments()[0] {
        0x01 => Ok(1000),
        0x02 => Ok(500),
        0x08 => Ok(125),
        _ => Err(anyhow!("Invalid polling rate response")),
    }
}

async fn get_polling_rate_0x3f(interface: Interface) -> Result<u16> {
    get_polling_rate(interface, 0x3f).await
}

async fn set_polling_rate(
    interface: Interface,
    polling_rate: PollingRate,
    transaction_id: u8,
) -> Result<()> {
    match polling_rate {
        PollingRate::Normal(polling_rate) => {
            let request = RazerMessageBuilder::set_polling_rate(polling_rate)
                .with_transaction_id(transaction_id)
                .build();
            send_razer_message(interface, request).await
        }
        PollingRate::Extended(_) => Err(anyhow!(
            "Trying to use ExtendedPollingRate on a NormalPollingRate device."
        )),
    }
}

async fn set_polling_rate_0x3f(interface: Interface, polling_rate: PollingRate) -> Result<()> {
    set_polling_rate(interface, polling_rate, 0x3F).await
}

#[allow(unused)]
async fn set_polling_rate_0x1f(interface: Interface, polling_rate: PollingRate) -> Result<()> {
    set_polling_rate(interface, polling_rate, 0x1F).await
}

#[allow(unused)]
async fn set_polling_rate_extended(interface: Interface, polling_rate: PollingRate) -> Result<()> {
    match polling_rate {
        PollingRate::Normal(_) => Err(anyhow!(
            "Trying to use NormalPollingRate on an ExtendedPollingRate device."
        )),
        PollingRate::Extended(polling_rate) => {
            let request = RazerMessageBuilder::set_polling_rate_extended(polling_rate)
                .with_transaction_id(0x1f)
                .build();
            send_razer_message(interface, request).await
        }
    }
}

async fn get_battery_level(interface: Interface, transaction_id: u8) -> Result<f32> {
    let request = RazerMessageBuilder::get_battery_level()
        .with_transaction_id(transaction_id)
        .build();
    let response = send_razer_message_and_wait_response(interface, request).await?;

    let battery_level = response.arguments()[1] as f32 / 255. * 100.;
    Ok(battery_level)
}

async fn get_battery_level_0x3f(interface: Interface) -> Result<f32> {
    get_battery_level(interface, 0x3f).await
}

async fn get_charging_status(interface: Interface, transaction_id: u8) -> Result<bool> {
    let request = RazerMessageBuilder::get_charging_status()
        .with_transaction_id(transaction_id)
        .build();
    let response = send_razer_message_and_wait_response(interface, request).await?;

    let charging_status = response.arguments()[1] > 0;
    Ok(charging_status)
}

async fn get_charging_status_0x3f(interface: Interface) -> Result<bool> {
    get_charging_status(interface, 0x3f).await
}

async fn chroma_logo_matrix_effect(
    interface: Interface,
    effect: ExtendedMatrixEffect,
    transaction_id: u8,
) -> Result<()> {
    let request = RazerMessageBuilder::chroma_extended_matrix_effect(
        VarStoreId::VarStore,
        LedId::Logo,
        effect,
    )
    .with_transaction_id(transaction_id)
    .build();

    send_razer_message(interface, request).await
}

async fn chroma_logo_matrix_effect_0x3f(
    interface: Interface,
    effect: ExtendedMatrixEffect,
) -> Result<()> {
    chroma_logo_matrix_effect(interface, effect, 0x3f).await
}

device_impls!([
    DeathadderV2ProWireless 0x007D {
        get_dpi: get_dpi_0x3f,
        set_dpi: set_dpi_0x3f,
        get_dpi_stages: get_dpi_stages_0x3f,
        set_dpi_stages: set_dpi_stages_0x3f,
        get_polling_rate: get_polling_rate_0x3f,
        set_polling_rate: set_polling_rate_0x3f,
        get_battery_level: get_battery_level_0x3f,
        get_charging_status: get_charging_status_0x3f,
        chroma_logo_matrix_effect: chroma_logo_matrix_effect_0x3f,
    },
]);
