use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

/// Main system metrics structure that matches the OpenAPI schema
#[derive(Debug, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub timestamp: DateTime<Utc>,
    pub system: SystemInfo,
    pub cpu: CpuInfo,
    pub memory: MemoryInfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu: Option<Vec<GpuInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<NetworkInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thermal: Option<ThermalInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage: Option<StorageInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peripherals: Option<PeripheralsInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "appleSilicon")]
    pub apple_silicon: Option<AppleSiliconInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SystemInfo {
    pub hostname: String,
    pub platform: String,
    pub release: String,
    pub uptime: u64,
    pub loadavg: Vec<f64>,
    #[serde(rename = "isAppleSilicon")]
    pub is_apple_silicon: bool,
    pub model: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CpuInfo {
    pub info: CpuHardwareInfo,
    pub load: CpuLoadInfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<CpuTemperatureInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CpuHardwareInfo {
    pub manufacturer: String,
    pub brand: String,
    pub cores: CpuCoreCount,
    pub speed: CpuSpeed,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CpuCoreCount {
    pub physical: u32,
    pub logical: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CpuSpeed {
    pub base: f64,
    pub max: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current: Option<Vec<f64>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CpuLoadInfo {
    pub current: f64,
    pub user: f64,
    pub system: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cores: Option<Vec<CoreLoadInfo>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CoreLoadInfo {
    pub number: u32,
    pub load: f64,
    pub user: f64,
    pub system: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CpuTemperatureInfo {
    pub main: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cores: Option<Vec<f64>>,
    pub max: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MemoryInfo {
    pub total: u64,
    pub used: u64,
    pub active: u64,
    pub available: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub swap: Option<SwapInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SwapInfo {
    pub total: u64,
    pub used: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GpuInfo {
    pub model: String,
    pub vendor: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vram: Option<GpuVramInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GpuVramInfo {
    pub total: u64,
    pub used: u64,
    pub free: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NetworkInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interfaces: Option<Vec<NetworkInterface>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stats: Option<Vec<NetworkStats>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NetworkInterface {
    pub name: String,
    pub r#type: String,
    pub mac: String,
    pub ipv4: String,
    pub ipv6: String,
    pub speed: u64,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NetworkStats {
    pub interface: String,
    pub rx_sec: f64,
    pub tx_sec: f64,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub errors: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ThermalInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chassis: Option<ChassisTemperature>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub battery: Option<BatteryThermal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fan: Option<FanInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pressure: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChassisTemperature {
    pub temperature: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BatteryThermal {
    pub temperature: f64,
    pub health: f64,
    #[serde(rename = "cycleCount")]
    pub cycle_count: u32,
    #[serde(rename = "isCharging")]
    pub is_charging: bool,
    pub voltage: f64,
    pub percent: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FanInfo {
    pub speed: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StorageInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filesystems: Option<Vec<FilesystemInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub io: Option<IoInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FilesystemInfo {
    pub fs: String,
    pub r#type: String,
    pub size: u64,
    pub used: u64,
    pub available: u64,
    pub mount: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IoInfo {
    #[serde(rename = "totalRead")]
    pub total_read: u64,
    #[serde(rename = "totalWrite")]
    pub total_write: u64,
    #[serde(rename = "readBytesPerSec")]
    pub read_bytes_per_sec: f64,
    #[serde(rename = "writeBytesPerSec")]
    pub write_bytes_per_sec: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PeripheralsInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub changes: Option<PeripheralChanges>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PeripheralChanges {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub added: Option<PeripheralChangesByType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub removed: Option<PeripheralChangesByType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub changed: Option<PeripheralChangesByType>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PeripheralChangesByType {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usb: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bluetooth: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppleSiliconInfo {
    pub chip: AppleSiliconChip,
    pub power: AppleSiliconPower,
    pub thermal: AppleSiliconThermal,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppleSiliconChip {
    pub model: String,
    pub cores: AppleSiliconCores,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppleSiliconCores {
    pub cpu: u32,
    pub gpu: u32,
    pub neural_engine: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppleSiliconPower {
    pub cpu_power: f64,
    pub gpu_power: f64,
    pub package_power: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppleSiliconThermal {
    pub levels: AppleSiliconThermalLevels,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppleSiliconThermalLevels {
    pub cpu: u32,
    pub gpu: u32,
    pub io: u32,
}

// API response models
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse {
    pub success: bool,
    pub node: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub code: u32,
    pub message: String,
} 