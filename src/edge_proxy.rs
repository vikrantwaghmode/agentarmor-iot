use crate::hal::HalError;
use crate::policy::{ActionParameters, AgentAction, AgentArmorEdge, SafetyDecision, SystemState};
use crate::policy_config::PolicyConfig;
use core::str::FromStr;
use tracing::{error, info, warn};
use crate::device_manager::DeviceManager;

// The `EdgeProxy` is the central nervous system of the `agentarmor-iot` framework.
// It is responsible for:
// 1. Intercepting raw (and potentially unsafe) commands from the AI agent.
// 2. Deserializing and validating the command structure.
// 3. Forwarding the command to the `AgentArmorEdge` policy engine for evaluation.
// 4. Handling the `SafetyDecision` from the policy engine.
// 5. If the command is allowed or corrected, dispatching it to the appropriate hardware actuator.
// 6. Providing a "dry run" mode for testing and diagnostics.
pub struct EdgeProxy<'a, const MAX_GUARDRAILS: usize> {
    engine: AgentArmorEdge<'a, MAX_GUARDRAILS>,
    device_manager: DeviceManager<'a>,
}

impl<'a, const MAX_GUARDRAILS: usize> EdgeProxy<'a, MAX_GUARDRAILS> {
    pub fn new(engine: AgentArmorEdge<'a, MAX_GUARDRAILS>, device_manager: DeviceManager<'a>) -> Self {
        Self { engine, device_manager }
    }

    pub fn intercept_and_process(
        &mut self,
        policy: &PolicyConfig,
        state: &SystemState,
        raw_json_payload: &str,
        dry_run: bool,
    ) -> Option<SafetyDecision> {
        // Step 1: Deserialize the raw JSON payload into a structured AgentAction.
        let action: AgentAction = match serde_json::from_str(raw_json_payload) {
            Ok(action) => action,
            Err(e) => {
                error!("☣️ JSON Deserialization Failed: {}", e);
                // A malformed command, such as one with missing fields, is an attack vector.
                return Some(SafetyDecision::Block(
                    heapless::String::from_str("JSON deserialization failed").unwrap(),
                ));
            }
        };

        // Step 2: Enforce the policy against the action.
        let decision = self.engine.enforce(policy, state, &action);

        // Step 3: Handle the decision.
        match &decision {
            SafetyDecision::Allow => {
                info!("✅ LLM Output ALLOWED by policy engine.");
                if !dry_run {
                    self.dispatch_to_hardware(&action.target_hardware, &action.parameters);
                } else {
                    warn!("🌵 Dry Run: ALLOW (Hardware execution skipped)");
                }
            }
            SafetyDecision::Corrected(corrected_action) => {
                warn!("⚠️ LLM Output CORRECTED by policy engine.");
                if !dry_run {
                    self.dispatch_to_hardware(&corrected_action.target_hardware, &corrected_action.parameters);
                } else {
                    warn!("🌵 Dry Run: CORRECT (Hardware execution skipped)");
                }
            }
            SafetyDecision::Block(reason) => {
                error!("❌ LLM Output BLOCKED by policy engine: {}", reason);
                if !dry_run {
                    // As an extra safety measure, trigger an e-stop on all actuators
                    // if the AI tries to do something dangerous.
                    self.device_manager.emergency_stop_all();
                } else {
                    warn!("🌵 Dry Run: BLOCK (Hardware execution skipped)");
                }
            }
        }
        Some(decision)
    }

    // Helper function to dispatch the command to the hardware.
    fn dispatch_to_hardware(&mut self, target_hardware: &str, params: &ActionParameters) {
        if let Err(e) = self.device_manager.dispatch(target_hardware, params) {
            match e {
                HalError::DeviceNotFound(device) => error!("HAL Error: Device '{}' not found.", device),
                HalError::InvalidAction(action) => error!("HAL Error: Invalid action '{}'.", action),
                HalError::HardwareFault(fault) => error!("HAL Error: Hardware fault '{}'.", fault),
            }
        }
    }
}
