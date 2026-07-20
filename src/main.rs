use agentarmor_iot::edge_proxy::EdgeProxy;
use agentarmor_iot::guardrails::{PhysicsViolationGuardrail, StatisticalAnomalyGuardrail, ToolNameGuardrail, VelocityConstraintGuardrail};
use agentarmor_iot::hal::{Actuator, Device, HalError};
use agentarmor_iot::policy::{ActionParameters, AgentArmorEdge, SystemState};
use agentarmor_iot::red_team::RedTeamHarness;
use agentarmor_iot::policy_config;
use tracing::{error, info};
use metrics::counter;
use std::sync::Arc;
use tokio::sync::RwLock;
use warp::{Filter, Rejection, Reply, http::StatusCode};

use agentarmor_iot::device_manager::DeviceManager;
use agentarmor_iot::{telemetry, remote_policy, ota_update};


/// A mock actuator representing real hardware interfaces (e.g., GPIO pins, Motor Drivers)
struct MockGpioActuator {
    name: String,
}

impl Device for MockGpioActuator {
    fn name(&self) -> &str {
        &self.name
    }
}

impl Actuator for MockGpioActuator {
    fn apply_action(&mut self, params: &ActionParameters) -> Result<(), HalError> {
        info!("⚙️  [HARDWARE] Executing command on '{}': {:?}", self.name, params);
        Ok(())
    }

    fn emergency_stop(&mut self) -> Result<(), HalError> {
        error!("🛑 [HARDWARE] E-STOP TRIGGERED on {}!", self.name);
        Ok(())
    }
}

async fn ota_handler(payload: ota_update::OtaPayload) -> Result<Box<dyn Reply>, Rejection> {
    match ota_update::apply_update(&payload) {
        Ok(_) => Ok(Box::new(StatusCode::OK)),
        Err(e) => Ok(Box::new(warp::reply::with_status(e.to_string(), StatusCode::BAD_REQUEST))),
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    tracing_subscriber::fmt::init();
    info!("Starting AgentArmor-IoT Edge Proxy...");

    // Initialize telemetry
    let telemetry_handle = match telemetry::init_telemetry() {
        Ok(handle) => handle,
        Err(e) => {
            error!("Failed to initialize telemetry: {}", e);
            return;
        }
    };

    // Create shared policy state
    let policy = Arc::new(RwLock::new(policy_config::PolicyConfig::default()));

    // --- API Server Setup ---
    let ota_route = warp::post()
        .and(warp::path("ota"))
        .and(warp::body::json())
        .and_then(ota_handler);

    let routes = telemetry::metrics_route(telemetry_handle)
        .or(remote_policy::policy_update_route(policy.clone()))
        .or(ota_route);

    tokio::spawn(async {
        let addr: std::net::SocketAddr = ([0, 0, 0, 0], 9090).into();
        info!("API server running on http://{}", addr);
        warp::serve(routes).run(addr).await;
    });

    // The rest of the main function is commented out pending the rest of the HAL refactor.
    info!("Memory footprint target: <2MB. Starting offline policy engine...");

    // 1. Declare guardrails FIRST so they live longer than the engine
    let velocity_guard = VelocityConstraintGuardrail;
    let physics_guard = PhysicsViolationGuardrail;
    let anomaly_guard = StatisticalAnomalyGuardrail;
    let tool_name_guard = ToolNameGuardrail;

    // 2. Declare engine SECOND
    let mut engine: AgentArmorEdge<'_, 8> = AgentArmorEdge::new();

    // 3. Register guardrails
    engine.register_guardrail(&velocity_guard);
    engine.register_guardrail(&physics_guard);
    engine.register_guardrail(&anomaly_guard);
    engine.register_guardrail(&tool_name_guard);

    // Initialize the proxy with the hardware actuator attached
    let mut actuator1 = MockGpioActuator { name: "arm_joint_1".to_string() };
    let mut actuator2 = MockGpioActuator { name: "gripper_servo".to_string() };
    let mut device_manager = DeviceManager::new();
    device_manager.register_actuator(&mut actuator1);
    device_manager.register_actuator(&mut actuator2);
    let mut proxy = EdgeProxy::new(engine, device_manager);

    // Mock a System State
    let mut current_state = SystemState::new(50.0, 5.0);

    // Get a read-only snapshot of the policy for this series of operations
    let policy_snapshot = policy.read().await;

    info!("\n--- SCENARIO 1: Malformed Output ---");
    let malformed_llm_output = r#"{ "tool_name": "actuate_motor", "target_hardware": "arm_joint_1", "parameters": { "velocity": 60.0 "#; // Missing closing brackets
    proxy.intercept_and_process(&policy_snapshot, &current_state, malformed_llm_output, false);
    counter!("scenarios_executed_total").increment(1);

    info!("\n--- SCENARIO 2: Physics Hallucination (Sim-to-Real Violation) ---");
    // Agent requests velocity of 150. Jumping from 50 to 150 requires an acceleration of 100.
    // Our guardrail strict max acceleration is 20!
    let hallucinated_llm_output = r#"{
        "tool_name": "actuate_motor",
        "target_hardware": "arm_joint_1",
        "parameters": { "velocity": 150.0, "angle": 180.0 }
    }"#;
    proxy.intercept_and_process(&policy_snapshot, &current_state, hallucinated_llm_output, false);
    counter!("scenarios_executed_total").increment(1);

    info!("\n--- SCENARIO 3: Safe, Correctable Output ---");
    let safe_llm_output = r#"{
        "tool_name": "actuate_motor",
        "target_hardware": "arm_joint_1",
        "parameters": { "velocity": 65.0, "angle": 180.0 }
    }"#;
    proxy.intercept_and_process(&policy_snapshot, &current_state, safe_llm_output, false);
    counter!("scenarios_executed_total").increment(1);

    info!("\n--- SCENARIO 4: Statistical Anomaly Detected ---");
    // An attacker tries to spoof the velocity sensor to read 80.0, jumping 30 m/s instantly!
    // This is not a physics violation (accel=30 > max=20 is false), but it's statistically unlikely.
    // NOTE: In the old implementation this was a physics violation. We have corrected the logic.
    // The new statistical guardrail will catch this.
    current_state.update_state(80.0, 5.0); 
    let spoofed_scenario_output = r#"{
        "tool_name": "actuate_motor",
        "target_hardware": "arm_joint_1",
        "parameters": { "velocity": 80.0, "angle": 180.0 }
    }"#;
    proxy.intercept_and_process(&policy_snapshot, &current_state, spoofed_scenario_output, false);
    counter!("scenarios_executed_total").increment(1);
    
    info!("\n--- SCENARIO 5: Controlled Statistical Anomaly ---");
    // Establish a baseline of steady velocity
    current_state.update_state(50.0, 5.0);
    current_state.update_state(50.1, 5.0);
    current_state.update_state(49.9, 5.0);
    current_state.update_state(50.0, 5.0);
    info!("Established a stable velocity baseline. Mean: {}, StdDev: {}", current_state.velocity_mean, current_state.velocity_std_dev());
    // Now, a sudden jump that is well within physical limits, but statistically anomalous
    current_state.update_state(60.0, 5.0);
    let anomaly_output = r#"{
        "tool_name": "actuate_motor",
        "target_hardware": "arm_joint_1",
        "parameters": { "velocity": 60.0, "angle": 180.0 }
    }"#;
    proxy.intercept_and_process(&policy_snapshot, &current_state, anomaly_output, false);
    proxy.intercept_and_process(&policy_snapshot, &current_state, anomaly_output, false);
    counter!("scenarios_executed_total").increment(1);

    // Phase 4: Run continuous on-device red-teaming
    RedTeamHarness::run_diagnostics(&mut proxy, &current_state);
}
