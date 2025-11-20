use anyhow::Result;
use botticelli_server::{ModelManager, ModelSpec};
use clap::Parser;
use tracing::{info, Level};

#[derive(Parser, Debug)]
#[command(author, version, about = "Botticelli Local Inference Server", long_about = None)]
struct Args {
    /// Model to use (e.g., "mistral-7b-q4", "mistral-7b-q5", "mistral-7b-q8")
    #[arg(short, long)]
    model: Option<String>,

    /// Directory to download/store models (default: ./models)
    #[arg(short, long, default_value = "./models")]
    download_dir: String,

    /// Port to run the server on
    #[arg(short, long, default_value = "8080")]
    port: u16,

    /// List available models and exit
    #[arg(short, long)]
    list: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    let args = Args::parse();

    // Handle list command
    if args.list {
        println!("\nAvailable models:");
        println!("{:-<80}", "");
        for spec in ModelSpec::all() {
            println!("{}", spec.description());
            println!("  ID: mistral-7b-q{}", match spec {
                ModelSpec::Mistral7BInstructV03Q4 => "4",
                ModelSpec::Mistral7BInstructV03Q5 => "5",
                ModelSpec::Mistral7BInstructV03Q8 => "8",
            });
            println!("  Size: ~{}GB", spec.size_gb());
            println!();
        }
        return Ok(());
    }

    // Parse model spec
    let model_name = args.model.ok_or_else(|| anyhow::anyhow!("--model is required. Use --list to see available models."))?;
    let model_spec = ModelSpec::parse(&model_name)
        .ok_or_else(|| anyhow::anyhow!("Unknown model: {}. Use --list to see available models.", model_name))?;

    info!(
        model = ?model_spec,
        download_dir = %args.download_dir,
        port = args.port,
        "Starting Botticelli inference server"
    );

    // Create model manager and ensure model is downloaded
    let manager = ModelManager::new(&args.download_dir);
    
    println!("\n{}", "=".repeat(80));
    println!("Botticelli Local Inference Server");
    println!("{}", "=".repeat(80));
    println!("Model: {}", model_spec.description());
    println!("Download directory: {}", args.download_dir);
    println!("Port: {}", args.port);
    println!("{}", "=".repeat(80));
    println!();

    if !manager.is_downloaded(model_spec) {
        println!("📥 Model not found locally. Downloading (~{}GB)...", model_spec.size_gb());
        println!("This may take several minutes depending on your connection.");
        println!();
    }

    let model_path = manager.ensure_model(model_spec).await?;

    println!("✅ Model ready at: {}", model_path.display());
    println!();
    println!("🚀 Starting inference server on port {}...", args.port);
    println!("   Server will be available at: http://localhost:{}", args.port);
    println!();
    println!("Note: Actual server startup not yet implemented.");
    println!("      Use mistralrs-server directly for now:");
    println!();
    println!("  mistralrs-server --port {} \\", args.port);
    println!("    gguf -m {} \\", args.download_dir);
    println!("    -f {} \\", model_spec.filename());
    println!("    -t {}", model_spec.tokenizer_id());
    println!();

    Ok(())
}
