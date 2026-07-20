use crate::hal::{Actuator, HalError};
use crate::policy::ActionParameters;
use std::collections::HashMap;

/// A manager for all hardware devices.
pub struct DeviceManager<'a> {
    actuators: HashMap<String, &'a mut dyn Actuator>,
}

impl<'a> DeviceManager<'a> {
    pub fn new() -> Self {
        Self {
            actuators: HashMap::new(),
        }
    }

    pub fn register_actuator(&mut self, actuator: &'a mut impl Actuator) {
        self.actuators.insert(actuator.name().to_string(), actuator);
    }

    pub fn dispatch(&mut self, target_hardware: &str, params: &ActionParameters) -> Result<(), HalError> {
        if let Some(actuator) = self.actuators.get_mut(target_hardware) {
            actuator.apply_action(params)
        } else {
            let mut s = heapless::String::<32>::new();
            s.push_str(target_hardware).unwrap();
            Err(HalError::DeviceNotFound(s))
        }
    }

    pub fn emergency_stop_all(&mut self) {
        for actuator in self.actuators.values_mut() {
            let _ = actuator.emergency_stop();
        }
    }
}
