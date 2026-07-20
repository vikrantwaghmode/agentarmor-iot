use crate::policy::{AgentAction, PhysicalGuardrail, SafetyDecision, SystemState};
use crate::policy_config::PolicyConfig;
use core::fmt::Write;
use heapless::String;
use metrics::counter;

// --- Specific Guardrail Implementations ---

pub struct VelocityConstraintGuardrail;

impl PhysicalGuardrail for VelocityConstraintGuardrail {
    fn evaluate(&self, policy: &PolicyConfig, _state: &SystemState, action: &AgentAction) -> SafetyDecision {
        if !policy.velocity_constraint.enabled {
            return SafetyDecision::Allow;
        }

        if action.tool_name == "actuate_motor" {
            if let Some(velocity) = action.parameters.velocity {
                if velocity > policy.velocity_constraint.params.max_safe_velocity {
                    counter!("guardrail_triggered_total", "guardrail" => "velocity_constraint");
                    // Instead of just failing, dynamically correct to a safe output
                    let mut corrected_params = action.parameters.clone();
                    corrected_params.velocity = Some(policy.velocity_constraint.params.max_safe_velocity);

                    return SafetyDecision::Corrected(AgentAction {
                        tool_name: action.tool_name.clone(),
                        target_hardware: action.target_hardware.clone(),
                        parameters: corrected_params,
                    });
                }
            }
        }
        SafetyDecision::Allow
    }
}

pub struct PhysicsViolationGuardrail;

impl PhysicalGuardrail for PhysicsViolationGuardrail {
    fn evaluate(&self, policy: &PolicyConfig, state: &SystemState, action: &AgentAction) -> SafetyDecision {
        if !policy.physics_violation.enabled {
            return SafetyDecision::Allow;
        }

        if action.tool_name == "actuate_motor" {
            if let Some(requested_velocity) = action.parameters.velocity {
                
                // This guardrail assumes the action happens "instantaneously" for evaluation.
                // It checks if the jump from current to requested velocity is physically plausible.
                let required_acceleration = (requested_velocity - state.current_velocity).abs();

                if required_acceleration > policy.physics_violation.params.max_acceleration {
                    counter!("guardrail_triggered_total", "guardrail" => "physics_violation");
                    let mut reason = String::<256>::new();
                    let _ = write!(reason, "PHYSICS VIOLATION: Requested velocity delta {}, max accel {}", required_acceleration, policy.physics_violation.params.max_acceleration);
                    return SafetyDecision::Block(reason);
                }
            }
        }
        SafetyDecision::Allow
    }
}

pub struct StatisticalAnomalyGuardrail;

impl PhysicalGuardrail for StatisticalAnomalyGuardrail {
    fn evaluate(&self, policy: &PolicyConfig, state: &SystemState, _action: &AgentAction) -> SafetyDecision {
        if !policy.statistical_anomaly.enabled {
            return SafetyDecision::Allow;
        }

        let std_dev = state.velocity_std_dev();
        if std_dev < 1e-6 { // Avoid division by zero or near-zero
            return SafetyDecision::Allow;
        }
        
        let z_score = ((state.current_velocity - state.velocity_mean) / std_dev).abs();

        if z_score > policy.statistical_anomaly.params.z_score_threshold {
            counter!("guardrail_triggered_total", "guardrail" => "statistical_anomaly");
            let mut reason = String::<256>::new();
            let _ = write!(reason, "STATISTICAL ANOMALY: Velocity Z-score {} exceeds threshold {}", z_score, policy.statistical_anomaly.params.z_score_threshold);
            return SafetyDecision::Block(reason);
        }
        
        SafetyDecision::Allow
    }
}

pub struct ToolNameGuardrail;

impl PhysicalGuardrail for ToolNameGuardrail {
    fn evaluate(&self, policy: &PolicyConfig, _state: &SystemState, action: &AgentAction) -> SafetyDecision {
        if !policy.allowed_tools.enabled {
            return SafetyDecision::Allow;
        }

        if !policy.allowed_tools.params.allowed_tool_names.contains(&action.tool_name) {
            counter!("guardrail_triggered_total", "guardrail" => "unknown_tool_name");
            let mut reason = String::<256>::new();
            let _ = write!(reason, "UNKNOWN TOOL: Tool '{}' is not in the list of allowed tools.", action.tool_name);
            return SafetyDecision::Block(reason);
        }

        SafetyDecision::Allow
    }
}