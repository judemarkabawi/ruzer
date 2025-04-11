use crate::{
    common::{Dpi, DpiStages, PollingRate, RAZER_MOUSE_WAIT_TIME},
    devices::FeatureSet,
};

#[derive(Clone, Debug, Default)]
pub struct DeviceInfo {
    pub dpi: Option<Dpi>,
    pub dpi_range: (u16, u16),
    pub dpi_stages: Option<DpiStages>,
    pub polling_rate: Option<PollingRate>,
    pub battery_level: Option<f32>,
    pub charging_status: Option<bool>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DeviceSettings {
    pub dpi: Option<Dpi>,
    pub dpi_stages: Option<DpiStages>,
    pub polling_rate: Option<PollingRate>,
}

#[allow(async_fn_in_trait)]
pub trait BatchedFeatureSet {
    async fn get_batched(&self) -> DeviceInfo;
    async fn set_batched(&self, settings: &DeviceSettings) -> anyhow::Result<()>;
}

impl BatchedFeatureSet for dyn FeatureSet {
    async fn get_batched(&self) -> DeviceInfo {
        // Sometimes the device returns garbage info (like DPI of 0) if set_batched() or get_batched() are called in quick succession
        tokio::time::sleep(RAZER_MOUSE_WAIT_TIME).await;
        let dpi = self.get_dpi().await;
        let dpi_range = self.get_dpi_range();
        let dpi_stages = self.get_dpi_stages().await;
        let polling_rate = self.get_polling_rate().await;
        let battery_level = self.get_battery_level().await;
        let charging_status = self.get_charging_status().await;

        DeviceInfo {
            dpi: dpi.ok(),
            dpi_range,
            dpi_stages: dpi_stages.ok(),
            polling_rate: polling_rate.ok(),
            battery_level: battery_level.ok(),
            charging_status: charging_status.ok(),
        }
    }

    async fn set_batched(&self, batched: &DeviceSettings) -> anyhow::Result<()> {
        tokio::time::sleep(RAZER_MOUSE_WAIT_TIME).await;

        if let Some(dpi) = batched.dpi {
            self.set_dpi(dpi).await?;
        }
        if let Some(dpi_stages) = &batched.dpi_stages {
            self.set_dpi_stages(dpi_stages).await?;
        }
        if let Some(polling_rate) = batched.polling_rate {
            self.set_polling_rate(polling_rate).await?;
        }
        Ok(())
    }
}
