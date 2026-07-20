use serde::{Deserialize, Serialize};
use heapless::Vec;
use core::fmt::Write;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct PolicyConfig {
    #[serde(default)]
    pub velocity_constraint: GuardrailConfig<VelocityConfig>,
    #[serde(default)]
    pub physics_violation: GuardrailConfig<PhysicsConfig>,
    #[serde(default)]
    pub sensor_spoofing: GuardrailConfig<SpoofingConfig>,
    #[serde(default)]
    pub statistical_anomaly: GuardrailConfig<StatisticalAnomalyConfig>,
    #[serde(default)]
    pub allowed_tools: GuardrailConfig<AllowedToolsConfig>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct GuardrailConfig<T> {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(flatten)]
    pub params: T,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct VelocityConfig {
    pub max_safe_velocity: f64,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct PhysicsConfig {
    pub max_acceleration: f64,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct SpoofingConfig {
    pub max_variance_threshold: f64,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct StatisticalAnomalyConfig {
    pub z_score_threshold: f64,
}

impl Default for StatisticalAnomalyConfig {
    fn default() -> Self {
        Self { z_score_threshold: 3.0 }
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct AllowedToolsConfig {
    pub allowed_tool_names: Vec<heapless::String<32>, 4>, // Allow up to 4 tool names
}

impl Default for AllowedToolsConfig {
    fn default() -> Self {
        let mut allowed_tool_names = Vec::new();
        let mut s = heapless::String::new();
        write!(s, "actuate_motor").unwrap();
        allowed_tool_names.push(s).unwrap();
        Self { allowed_tool_names }
    }
}

fn default_enabled() -> bool {
    true
}

impl<T: Default> Default for GuardrailConfig<T> {
    fn default() -> Self {
        Self {
            enabled: true,
            params: T::default(),
        }
    }
}

impl Default for PolicyConfig {
    fn default() -> Self {
        Self {
            velocity_constraint: GuardrailConfig {
                enabled: true,
                params: VelocityConfig { max_safe_velocity: 100.0 },
            },
            physics_violation: GuardrailConfig {
                enabled: true,
                params: PhysicsConfig { max_acceleration: 20.0 },
            },
            sensor_spoofing: GuardrailConfig {
                enabled: true,
                params: SpoofingConfig { max_variance_threshold: 15.0 },
            },
            statistical_anomaly: GuardrailConfig {
                enabled: true,
                params: StatisticalAnomalyConfig { z_score_threshold: 3.0 },
            },
            allowed_tools: GuardrailConfig::default(),
        }
    }
}
