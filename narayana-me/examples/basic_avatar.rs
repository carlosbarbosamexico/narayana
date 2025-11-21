//! Basic avatar example

use narayana_me::{AvatarConfig, AvatarBroker, AvatarProviderType};
use narayana_core::Error;

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Initialize tracing for better error messages
    tracing_subscriber::fmt::init();

    println!("Creating avatar broker...");

    // Create avatar config with Beyond Presence provider
    let mut config = AvatarConfig::default();
    config.enabled = true;
    config.provider = AvatarProviderType::BeyondPresence;
    config.enable_lip_sync = true;
    config.enable_gestures = true;
    config.expression_sensitivity = 0.8;

    // Create broker
    let broker = AvatarBroker::new(config)
        .map_err(|e| Error::Storage(format!("Failed to create broker: {}", e)))?;

    println!("Initializing avatar broker...");
    broker.initialize().await
        .map_err(|e| Error::Storage(format!("Failed to initialize: {}", e)))?;

    println!("Starting avatar stream...");
    match broker.start_stream().await {
        Ok(client_url) => {
            println!("✓ Avatar stream started successfully!");
            println!("  Client URL: {}", client_url);
        }
        Err(e) => {
            println!("✗ Failed to start stream: {}", e);
            println!("  (This is expected if Beyond Presence API is not available)");
        }
    }

    println!("\nAvatar broker is ready!");
    println!("You can now:");
    println!("  - Set expressions: broker.set_expression(...)");
    println!("  - Send audio: broker.send_audio(...)");
    println!("  - Set gestures: broker.set_gesture(...)");

    // Test setting an expression
    println!("\nTesting expression...");
    if let Err(e) = broker.set_expression(narayana_me::Expression::Happy, 0.8).await {
        println!("  Warning: Failed to set expression: {}", e);
    } else {
        println!("  ✓ Expression set successfully");
    }

    // Test emotion update
    println!("\nTesting emotion update...");
    if let Err(e) = broker.update_emotion(narayana_me::Emotion::Joy, 0.7).await {
        println!("  Warning: Failed to update emotion: {}", e);
    } else {
        println!("  ✓ Emotion updated successfully");
    }

    println!("\nExample completed!");
    Ok(())
}

