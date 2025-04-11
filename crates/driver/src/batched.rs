use crate::{
    common::{Dpi, DpiStages, PollingRate},
    devices::FeatureSet,
};

#[derive(Debug, Default)]
pub struct DeviceInfo {
    pub dpi: Option<Dpi>,
    pub dpi_stages: Option<DpiStages>,
    pub polling_rate: Option<PollingRate>,
    pub battery_level: Option<f32>,
    pub charging_status: Option<bool>,
}

#[allow(async_fn_in_trait)]
pub trait BatchedFeatureSet {
    type BatchedGet;
    async fn get_batched(&self) -> Self::BatchedGet;
}

impl BatchedFeatureSet for dyn FeatureSet {
    type BatchedGet = DeviceInfo;

    async fn get_batched(&self) -> DeviceInfo {
        let dpi = self.get_dpi().await;
        let dpi_stages = self.get_dpi_stages().await;
        let polling_rate = self.get_polling_rate().await;
        let battery_level = self.get_battery_level().await;
        let charging_status = self.get_charging_status().await;

        DeviceInfo {
            dpi: dpi.ok(),
            dpi_stages: dpi_stages.ok(),
            polling_rate: polling_rate.ok(),
            battery_level: battery_level.ok(),
            charging_status: charging_status.ok(),
        }
    }
}
