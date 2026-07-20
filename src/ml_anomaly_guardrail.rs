use crate::policy::{PhysicalGuardrail, SafetyDecision, SystemState, AgentAction};
use crate::policy_config::PolicyConfig;
use micrograd::{Value, MLP};
use tracing::info;

/// A guardrail that uses a simple machine learning model (an autoencoder)
/// to detect anomalous system states.
pub struct MLAnomalyGuardrail {
    autoencoder: MLP,
}

impl MLAnomalyGuardrail {
    pub fn new() -> Self {
        // For this PoC, we are using a "pre-trained" autoencoder with hardcoded weights.
        // In a real-world scenario, these weights would be the result of an offline
        // training process on a large dataset of normal system states.
        // The architecture is 2-2-2:
        // - Input layer: 2 neurons (velocity, proximity)
        // - Hidden layer: 2 neurons
        // - Output layer: 2 neurons (reconstructed velocity, reconstructed proximity)
        let autoencoder = MLP::new(2, vec![2, 2]);

        // Manually setting the weights and biases to "simulate" a trained model.
        // These values are chosen to create a simple identity-like function for a
        // specific range of inputs.
        // Note: this is a highly simplified for demonstration purposes.
        if let Some(layer) = autoencoder.layers().get(0) {
            if let Some(neuron) = layer.neurons().get(0) {
                neuron.w[0].set_data(1.0);
                neuron.w[1].set_data(0.0);
                neuron.b.set_data(0.0);
            }
            if let Some(neuron) = layer.neurons().get(1) {
                neuron.w[0].set_data(0.0);
                neuron.w[1].set_data(1.0);
                neuron.b.set_data(0.0);
            }
        }
        if let Some(layer) = autoencoder.layers().get(1) {
            if let Some(neuron) = layer.neurons().get(0) {
                neuron.w[0].set_data(1.0);
                neuron.w[1].set_data(0.0);
                neuron.b.set_data(0.0);
            }
           if let Some(neuron) = layer.neurons().get(1) {
                neuron.w[0].set_data(0.0);
                neuron.w[1].set_data(1.0);
                neuron.b.set_data(0.0);
            }
        }


        Self { autoencoder }
    }

    // A helper function to normalize the inputs.
    // ML models work best with normalized data, typically in the [0, 1] range.
    fn normalize_state(state: &SystemState) -> (f64, f64) {
        // These normalization factors would be determined during training.
        // For now, we'll use some reasonable estimates.
        let norm_velocity = state.current_velocity / 100.0; // Assume max velocity is 100
        let norm_proximity = state.proximity_sensor_distance / 10.0; // Assume max proximity is 10
        (norm_velocity.max(0.0).min(1.0), norm_proximity.max(0.0).min(1.0))
    }
}

impl PhysicalGuardrail for MLAnomalyGuardrail {
    fn evaluate(
        &self,
        _policy: &PolicyConfig,
        current_state: &SystemState,
        _action: &AgentAction,
    ) -> SafetyDecision {
        // 1. Normalize the input state
        let (norm_velocity, norm_proximity) = Self::normalize_state(current_state);
        let inputs = vec![Value::from(norm_velocity), Value::from(norm_proximity)];

        // 2. Pass the state through the autoencoder to get the reconstructed state
        let outputs = self.autoencoder.forward(inputs);
        let reconstructed_velocity = outputs[0].data();
        let reconstructed_proximity = outputs[1].data();

        // 3. Calculate the reconstruction error (Mean Squared Error)
        let error = ((norm_velocity - reconstructed_velocity).powi(2)
            + (norm_proximity - reconstructed_proximity).powi(2))
            / 2.0;

        // 4. Compare the error to a threshold
        // This threshold would also be determined during model validation.
        let anomaly_threshold = 0.1;

        info!(
            "🧠 ML Anomaly Guardrail: Input=({:.2}, {:.2}), Reconstructed=({:.2}, {:.2}), Error={:.4}",
            norm_velocity, norm_proximity, reconstructed_velocity, reconstructed_proximity, error
        );

        if error > anomaly_threshold {
            let reason = heapless::String::from("ML model detected anomalous system state");
            SafetyDecision::Block(reason)
        } else {
            SafetyDecision::Allow
        }
    }
}
