use agentarmor_iot::device_manager::DeviceManager;
use agentarmor_iot::hal::{Actuator, Device, HalError};
use agentarmor_iot::policy::ActionParameters;
use std::cell::RefCell;

struct MockActuator {
    name: String,
    action_log: RefCell<Vec<ActionParameters>>,
    emergency_stop_log: RefCell<Vec<()>>,
}

impl Device for MockActuator {
    fn name(&self) -> &str {
        &self.name
    }
}

impl Actuator for MockActuator {
    fn apply_action(&mut self, params: &ActionParameters) -> Result<(), HalError> {
        self.action_log.borrow_mut().push(params.clone());
        Ok(())
    }

    fn emergency_stop(&mut self) -> Result<(), HalError> {
        self.emergency_stop_log.borrow_mut().push(());
        Ok(())
    }
}

#[test]
fn test_device_manager_dispatch() {
    let mut actuator = MockActuator {
        name: "test_actuator".to_string(),
        action_log: RefCell::new(Vec::new()),
        emergency_stop_log: RefCell::new(Vec::new()),
    };
    let mut device_manager = DeviceManager::new();
    device_manager.register_actuator(&mut actuator);

    let params = ActionParameters {
        velocity: Some(10.0),
        angle: Some(90.0),
        ignore_safety: None,
    };
    device_manager.dispatch("test_actuator", &params).unwrap();

    let action_log = actuator.action_log.borrow();
    assert_eq!(action_log.len(), 1);
    assert_eq!(action_log[0], params);
}

#[test]
fn test_device_manager_dispatch_not_found() {
    let mut device_manager = DeviceManager::new();
    let params = ActionParameters {
        velocity: Some(10.0),
        angle: Some(90.0),
        ignore_safety: None,
    };
    let result = device_manager.dispatch("unknown_actuator", &params);
    assert!(matches!(result, Err(HalError::DeviceNotFound(_))));
}

#[test]
fn test_device_manager_emergency_stop() {
    let mut actuator1 = MockActuator {
        name: "actuator1".to_string(),
        action_log: RefCell::new(Vec::new()),
        emergency_stop_log: RefCell::new(Vec::new()),
    };
    let mut actuator2 = MockActuator {
        name: "actuator2".to_string(),
        action_log: RefCell::new(Vec::new()),
        emergency_stop_log: RefCell::new(Vec::new()),
    };
    let mut device_manager = DeviceManager::new();
    device_manager.register_actuator(&mut actuator1);
    device_manager.register_actuator(&mut actuator2);

    device_manager.emergency_stop_all();

    assert_eq!(actuator1.emergency_stop_log.borrow().len(), 1);
    assert_eq!(actuator2.emergency_stop_log.borrow().len(), 1);
}
