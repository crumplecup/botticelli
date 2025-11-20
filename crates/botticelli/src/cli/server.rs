//! Server management command handlers.

use crate::cli::ServerCommands;
use std::path::PathBuf;

/// Handle server management commands
pub async fn handle_server_command(cmd: ServerCommands) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        ServerCommands::Download {
            model,
            model_dir,
            quantization,
        } => {
            download_model(&model, &model_dir, &quantization).await?;
        }

        ServerCommands::Start {
            model,
            model_dir,
            port,
            daemon,
        } => {
            start_server(&model, &model_dir, port, daemon).await?;
        }

        ServerCommands::Stop => {
            stop_server().await?;
        }

        ServerCommands::Status => {
            check_status().await?;
        }

        ServerCommands::List { model_dir } => {
            list_models(&model_dir).await?;
        }
    }

    Ok(())
}

async fn download_model(
    model: &str,
    model_dir: &std::path::Path,
    quantization: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use botticelli_server::{ModelManager, ModelSpec};

    println!("Downloading model: {} ({})", model, quantization);
    println!("Target directory: {}", model_dir.display());

    let spec = ModelSpec::from_name(model)?;
    let manager = ModelManager::new(model_dir);
    let path = manager.download(&spec, quantization).await?;

    println!("✓ Model downloaded successfully to: {}", path.display());
    println!("\nTo start the server, run:");
    println!("  botticelli server start {}", model);

    Ok(())
}

async fn start_server(
    model: &str,
    model_dir: &std::path::Path,
    port: u16,
    daemon: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use botticelli_server::{ModelManager, ModelSpec, ServerHandle, ServerConfig};

    println!("Starting server with model: {}", model);
    println!("Model directory: {}", model_dir.display());
    println!("Port: {}", port);

    let spec = ModelSpec::from_name(model)?;
    let manager = ModelManager::new(model_dir);
    let model_path = manager.get_model_path(&spec, "q4");

    if !model_path.exists() {
        println!("Model not found. Downloading...");
        manager.download(&spec, "q4").await?;
    }

    let config = ServerConfig::new(format!("http://localhost:{}", port), model);
    let handle = ServerHandle::start(config, model_path, port)?;

    if daemon {
        println!("✓ Server started in background on port {}", port);
        println!("  Check status with: botticelli server status");
        std::mem::forget(handle); // Keep running in background
    } else {
        println!("Server running on port {} (press Ctrl+C to stop)...", port);
        
        // Wait for Ctrl+C signal
        tokio::signal::ctrl_c().await?;
        
        println!("\nShutting down server...");
        handle.stop()?;
        println!("✓ Server stopped");
    }

    Ok(())
}

async fn stop_server() -> Result<(), Box<dyn std::error::Error>> {
    println!("Stop functionality not yet implemented.");
    println!("Use `pkill mistralrs` or find the process with `ps aux | grep mistralrs`");
    Ok(())
}

async fn check_status() -> Result<(), Box<dyn std::error::Error>> {
    use botticelli_server::ServerClient;

    let config = botticelli_server::ServerConfig::new("http://localhost:8080", "default");
    let client = ServerClient::new(config);

    match client.health_check().await {
        Ok(_) => {
            println!("✓ Server is running on http://localhost:8080");
        }
        Err(e) => {
            println!("✗ Server is not responding: {}", e);
        }
    }

    Ok(())
}

async fn list_models(model_dir: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    use std::fs;

    println!("Available models in {}:", model_dir.display());
    println!();

    // List downloaded models
    if model_dir.exists() {
        let entries = fs::read_dir(model_dir)?;
        let mut found_any = false;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("gguf") {
                found_any = true;
                println!("  • {}", path.file_name().unwrap().to_string_lossy());
            }
        }

        if !found_any {
            println!("  (no models downloaded yet)");
        }
    } else {
        println!("  (directory does not exist)");
    }

    println!();
    println!("Popular models you can download:");
    println!("  • mistral-7b-instruct (or mistral) - Mistral 7B Instruct v0.3 Q4 (~4GB)");
    println!("  • mistral-7b-q5                    - Mistral 7B Instruct v0.3 Q5 (~5GB)");
    println!("  • mistral-7b-q8                    - Mistral 7B Instruct v0.3 Q8 (~7GB)");
    println!();
    println!("Download with: botticelli server download <model>");

    Ok(())
}
