#![cfg(test)]
use agentarmor_iot::policy::{SystemState};



#[test]
fn test_system_state_new() {
    let state = SystemState::new(10.0, 5.0);
    assert_eq!(state.current_velocity, 10.0);
    assert_eq!(state.previous_velocity, 10.0);
    assert_eq!(state.proximity_sensor_distance, 5.0);
    assert_eq!(state.velocity_history_count, 1);
    assert_eq!(state.velocity_mean, 10.0);
    assert_eq!(state.velocity_m2, 0.0);
}

#[test]
fn test_system_state_update() {
    let mut state = SystemState::new(10.0, 5.0);
    state.update_state(20.0, 3.0);

    assert_eq!(state.current_velocity, 20.0);
    assert_eq!(state.previous_velocity, 10.0);
    assert_eq!(state.proximity_sensor_distance, 3.0);
    assert_eq!(state.velocity_history_count, 2);
    assert_eq!(state.velocity_mean, 15.0);
    assert_eq!(state.velocity_m2, 50.0);
}

#[test]
fn test_system_state_variance_std_dev() {
    let mut state = SystemState::new(10.0, 5.0);
    state.update_state(20.0, 3.0);
    state.update_state(15.0, 4.0);

    // Mean is (10 + 20 + 15) / 3 = 15
    // Variance is ((10-15)^2 + (20-15)^2 + (15-15)^2) / (3-1) = (25 + 25 + 0) / 2 = 25
    assert_eq!(state.velocity_variance(), 25.0);
    assert_eq!(state.velocity_std_dev(), 5.0);
}
