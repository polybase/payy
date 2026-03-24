//! Basic usage example for json-store
//!
//! This example demonstrates how to use the JsonStore for managing application state
//! with atomic JSON file persistence.
//!
//! Run with: cargo run --example basic_usage

use json_store::{JsonStore, JsonStoreError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct AppConfig {
    version: String,
    debug_mode: bool,
    max_connections: u32,
    server_settings: ServerSettings,
    user_preferences: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ServerSettings {
    host: String,
    port: u16,
    timeout_seconds: u64,
}

#[tokio::main]
async fn main() -> Result<(), JsonStoreError> {
    println!("🚀 JSON Store Example");
    println!("================================");

    // Create a store for our application configuration
    let config_store = JsonStore::<AppConfig>::new("./test_fixtures", "app_config.json").await?;

    // Display initial state
    let initial_config = config_store.get().await;
    println!("📋 Initial configuration:");
    println!("{initial_config:#?}");
    println!();

    // Update configuration using the update method
    println!("🔧 Updating configuration...");
    config_store
        .update(|config| {
            config.version = "1.0.0".to_string();
            config.debug_mode = true;
            config.max_connections = 100;

            // Update server settings
            config.server_settings.host = "localhost".to_string();
            config.server_settings.port = 8080;
            config.server_settings.timeout_seconds = 30;

            // Add user preferences
            config
                .user_preferences
                .insert("theme".to_string(), "dark".to_string());
            config
                .user_preferences
                .insert("language".to_string(), "en".to_string());
        })
        .await?;

    // Display updated configuration
    let updated_config = config_store.get().await;
    println!("✅ Updated configuration:");
    println!("{updated_config:#?}");
    println!();

    // Demonstrate incremental updates
    println!("📝 Making incremental updates...");
    for i in 1..=5 {
        config_store
            .update(|config| {
                config
                    .user_preferences
                    .insert(format!("setting_{i}"), format!("value_{i}"));
            })
            .await?;
    }

    // Show final state
    let final_config = config_store.get().await;
    println!("🎯 Final configuration:");
    println!("{final_config:#?}");
    println!();

    // Demonstrate complete replacement
    println!("🔄 Replacing entire configuration...");
    let new_config = AppConfig {
        version: "2.0.0".to_string(),
        debug_mode: false,
        max_connections: 200,
        server_settings: ServerSettings {
            host: "0.0.0.0".to_string(),
            port: 9090,
            timeout_seconds: 60,
        },
        user_preferences: {
            let mut prefs = HashMap::new();
            prefs.insert("theme".to_string(), "light".to_string());
            prefs.insert("auto_save".to_string(), "true".to_string());
            prefs
        },
    };

    config_store.set(new_config).await?;

    let replaced_config = config_store.get().await;
    println!("🔄 Replaced configuration:");
    println!("{replaced_config:#?}");
    println!();

    // Demonstrate persistence by creating a new store instance
    println!("💾 Testing persistence...");
    let second_store = JsonStore::<AppConfig>::new("./test_fixtures", "app_config.json").await?;
    let loaded_config = second_store.get().await;
    println!("📂 Loaded from file:");
    println!("{loaded_config:#?}");
    println!();

    // Show file path
    println!("📄 Configuration saved to: {:?}", config_store.file_path());

    println!("✨ Example completed successfully!");

    Ok(())
}
