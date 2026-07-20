use crate::policy::ActionParameters;
use heapless::String;

#[derive(Debug)]
pub enum HalError {
    DeviceNotFound(String<32>),
    InvalidAction(String<64>),
    HardwareFault(String<64>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum SensorValue {
    Velocity(f64),
    Distance(f64),
    Temperature(f64),
}

/// A generic trait for any hardware device.
pub trait Device {
    fn name(&self) -> &str;
}

/// A trait for a hardware device that can be actuated.
pub trait Actuator: Device {
    /// Applies a set of parameters to the actuator.
    fn apply_action(&mut self, params: &ActionParameters) -> Result<(), HalError>;
    
    /// Triggers an immediate physical halt.
    fn emergency_stop(&mut self) -> Result<(), HalError>;
}

/// A trait for a hardware device that can provide sensor readings.
pub trait Sensor: Device {
    fn read(&self) -> Result<SensorValue, HalError>;
}