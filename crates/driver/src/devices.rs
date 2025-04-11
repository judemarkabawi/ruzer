use std::ops::Deref;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use driver_macros::device_impl;
use nusb::{DeviceInfo, Interface};

use crate::{
    chroma::{ExtendedMatrixEffect, LedId},
    common::{
        decode_u16_from_bytes, send_razer_message, send_razer_message_and_wait_response, DpiStages,
        RazerMessageBuilder, VarStoreId, RAZER_USB_INTERFACE_NUMBER,
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

async fn get_dpi(interface: Interface) -> Result<(u16, u16)> {
    let request = RazerMessageBuilder::get_dpi(VarStoreId::NoStore)
        .with_transaction_id(0x3F)
        .build();
    let response = send_razer_message_and_wait_response(interface, request).await?;

    let dpi_x: u16 = decode_u16_from_bytes(&response.arguments()[1..=2]);
    let dpi_y: u16 = decode_u16_from_bytes(&response.arguments()[3..=4]);
    Ok((dpi_x, dpi_y))
}

async fn set_dpi(interface: Interface, (dpi_x, dpi_y): (u16, u16)) -> Result<()> {
    let request = RazerMessageBuilder::set_dpi(VarStoreId::NoStore, (dpi_x, dpi_y))
        .with_transaction_id(0x3F)
        .build();
    send_razer_message(interface, request).await
}

async fn get_dpi_stages(interface: Interface) -> Result<DpiStages> {
    let request = RazerMessageBuilder::get_dpi_stages(VarStoreId::VarStore)
        .with_transaction_id(0x3F)
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

async fn set_dpi_stages(interface: Interface, dpi_stages: &DpiStages) -> Result<()> {
    let request = RazerMessageBuilder::set_dpi_stages(VarStoreId::VarStore, dpi_stages)
        .with_transaction_id(0x3f)
        .build();
    send_razer_message(interface, request).await
}

async fn get_polling_rate(interface: Interface) -> Result<u16> {
    let request = RazerMessageBuilder::get_polling_rate()
        .with_transaction_id(0x3F)
        .build();
    let response = send_razer_message_and_wait_response(interface, request).await?;

    match response.arguments()[0] {
        0x01 => Ok(1000),
        0x02 => Ok(500),
        0x08 => Ok(125),
        _ => Err(anyhow!("Invalid polling rate response")),
    }
}

async fn get_battery_level(interface: Interface) -> Result<f32> {
    let request = RazerMessageBuilder::get_battery_level()
        .with_transaction_id(0x3F)
        .build();
    let response = send_razer_message_and_wait_response(interface, request).await?;

    let battery_level = response.arguments()[1] as f32 / 255. * 100.;
    Ok(battery_level)
}

async fn get_charging_status(interface: Interface) -> Result<bool> {
    let request = RazerMessageBuilder::get_charging_status()
        .with_transaction_id(0x3F)
        .build();
    let response = send_razer_message_and_wait_response(interface, request).await?;

    let charging_status = response.arguments()[1] > 0;
    Ok(charging_status)
}

async fn chroma_logo_matrix_effect(
    interface: Interface,
    effect: ExtendedMatrixEffect,
) -> Result<()> {
    let request = RazerMessageBuilder::chroma_extended_matrix_effect(
        VarStoreId::VarStore,
        LedId::Logo,
        effect,
    )
    .with_transaction_id(0x3F)
    .build();

    send_razer_message(interface, request).await
}

fn get_device_impl(product_id: u16, interface: Interface) -> Result<Box<dyn FeatureSet>> {
    match product_id {
        id if id == DEATHADDER_V2_PRO_WIRELESS => Ok(Box::new(DeathadderV2ProWireless(interface))),
        _ => Err(anyhow!("Unsupported device")),
    }
}

device_impl!(DEATHADDER_V2_PRO_WIRELESS 0x007D {
    get_dpi: get_dpi,
    set_dpi: set_dpi,
    get_dpi_stages: get_dpi_stages,
    set_dpi_stages: set_dpi_stages,
    get_polling_rate: get_polling_rate,
    get_battery_level: get_battery_level,
    get_charging_status: get_charging_status,
    chroma_logo_matrix_effect: chroma_logo_matrix_effect,
});
