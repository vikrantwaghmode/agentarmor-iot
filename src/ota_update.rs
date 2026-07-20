use serde::Deserialize;
use sha2::{Sha256, Digest};
use ed25519_dalek::{VerifyingKey, Signature, Verifier, Signer};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use tracing::{info, error, warn};

// The public key of the update server. In a real device, this would be burned into OTP memory.
const UPDATE_SERVER_PUBLIC_KEY_HEX: &str = "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";

#[derive(Deserialize, Debug)]
pub struct OtaPayload {
    /// Base64 encoded firmware image
    pub firmware_image: String,
    /// Hex encoded SHA256 checksum of the firmware image
    pub sha256_checksum: String,
    /// Base64 encoded ed25519 signature of the checksum
    pub signature: String,
}

/// Applies a firmware update after performing security checks.
///
/// 1. Verifies the SHA256 checksum of the image.
/// 2. Verifies the ED25519 signature of the checksum.
///
/// Returns Ok(()) if the update is valid and applied, otherwise an error string.
pub fn apply_update(payload: &OtaPayload) -> Result<(), &'static str> {
    // 1. Decode payload data
    let firmware = BASE64.decode(&payload.firmware_image).map_err(|_| "Failed to decode firmware image")?;
    let signature_bytes: [u8; 64] = BASE64.decode(&payload.signature)
        .map_err(|_| "Failed to decode signature")?
        .try_into()
        .map_err(|_| "Invalid signature length")?;
    let signature = Signature::from_bytes(&signature_bytes);
    
    // 2. Verify checksum
    info!("Verifying firmware checksum...");
    let mut hasher = Sha256::new();
    hasher.update(&firmware);
    let calculated_checksum = hasher.finalize();
    let provided_checksum = hex::decode(&payload.sha256_checksum).map_err(|_| "Invalid checksum format")?;

    if calculated_checksum.as_slice() != provided_checksum.as_slice() {
        error!("Checksum mismatch! Firmware may be corrupted.");
        return Err("Checksum mismatch");
    }
    info!("Checksum OK.");

    // 3. Verify signature
    info!("Verifying signature...");
    let public_key_bytes: [u8; 32] = hex::decode(UPDATE_SERVER_PUBLIC_KEY_HEX)
        .unwrap()
        .try_into()
        .unwrap(); // Should not fail
    let public_key = VerifyingKey::from_bytes(&public_key_bytes).unwrap(); // Should not fail

    if public_key.verify(provided_checksum.as_slice(), &signature).is_err() {
        error!("Invalid signature! Update is not from a trusted source.");
        return Err("Invalid signature");
    }
    info!("Signature OK.");

    // 4. Apply update (simulation)
    warn!("Applying OTA update... (simulated)");
    info!("New firmware size: {} bytes", firmware.len());
    // In a real system, you would write `firmware` to a standby partition
    // and configure the bootloader to switch to it on the next boot.
    
    info!("OTA update applied successfully. The device would now reboot.");
    Ok(())
}

/// A helper function to generate a valid OTA payload for testing.
/// This would be run on the build server, not the device.
#[allow(dead_code)]
pub fn generate_test_payload(firmware: &[u8], signing_key: &ed25519_dalek::SigningKey) -> String {
    let mut hasher = Sha256::new();
    hasher.update(firmware);
    let checksum = hasher.finalize();

    let signature: Signature = signing_key.sign(&checksum);

    let payload = serde_json::json!({
        "firmware_image": BASE64.encode(firmware),
        "sha256_checksum": hex::encode(checksum),
        "signature": BASE64.encode(signature.to_bytes()),
    });

    serde_json::to_string_pretty(&payload).unwrap()
}
