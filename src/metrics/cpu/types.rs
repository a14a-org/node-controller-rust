use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct CpuMetrics {
    pub node_id: String,
    pub collected_at: DateTime<Utc>,
    pub manufacturer: String,
    pub brand: String,
    pub physical_cores: u32,
    pub logical_cores: u32,
    pub base_speed: f64,
    pub max_speed: f64,
    pub current_load: f64,
    pub user_load: f64,
    pub system_load: f64,
    pub temperature_main: f64,
    pub temperature_max: f64,
    pub core_metrics: HashMap<String, CoreMetrics>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apple_silicon_data: Option<AppleSiliconData>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CoreMetrics {
    pub load: f64,
    pub user: f64,
    pub system: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppleSiliconData {
    pub chip: String,
    pub power: PowerMetrics,
    pub thermal: ThermalMetrics,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PowerMetrics {
    pub package_watts: f64,
    pub cpu_watts: f64,
    pub gpu_watts: f64,
    pub ane_watts: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ThermalMetrics {
    pub cpu_die: f64,
    pub gpu_die: f64,
    pub efficiency_cores: f64,
    pub performance_cores: f64,
} 