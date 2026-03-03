mod error;
mod manifest;
mod postprocess;
mod prompt;
mod provider;

use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand, ValueEnum};
use tracing_subscriber::EnvFilter;

use crate::error::SpriteGenError;
use crate::manifest::{find_missing_sprites, find_sprite};
use crate::postprocess::postprocess_image;
use crate::prompt::build_prompt;
use crate::provider::Provider;

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ProviderChoice {
    Openrouter,
    Openai,
}

#[derive(Parser)]
#[command(name = "sprite-gen", about = "AI-powered game sprite generator")]
struct Cli {
    /// Provider to use for image generation
    #[arg(long, value_enum, default_value_t = ProviderChoice::Openrouter)]
    provider: ProviderChoice,

    /// Project root directory (auto-detected if not specified)
    #[arg(long)]
    project_root: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List missing sprites
    List,
    /// Preview the prompt for a sprite (no API call)
    Preview {
        /// Sprite name
        name: String,
    },
    /// Generate a single sprite
    Generate {
        /// Sprite name
        name: String,
    },
    /// Generate all missing sprites
    Batch,
}

fn detect_project_root() -> PathBuf {
    let mut dir = std::env::current_dir().expect("failed to get current directory");
    loop {
        if dir.join("Cargo.toml").exists() && dir.join("assets").exists() {
            return dir;
        }
        if !dir.pop() {
            break;
        }
    }
    // Fallback: tools/sprite-gen が CWD の場合、2階層上がプロジェクトルート
    let cwd = std::env::current_dir().expect("failed to get current directory");
    if cwd.ends_with("tools/sprite-gen")
        && let Some(root) = cwd.parent().and_then(|p| p.parent())
    {
        return root.to_path_buf();
    }
    cwd
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    let project_root = cli.project_root.unwrap_or_else(detect_project_root);

    match cli.command {
        Command::List => cmd_list(&project_root),
        Command::Preview { name } => cmd_preview(&name)?,
        Command::Generate { name } => cmd_generate(&cli.provider, &project_root, &name).await?,
        Command::Batch => cmd_batch(&cli.provider, &project_root).await?,
    }

    Ok(())
}

fn cmd_list(project_root: &Path) {
    let missing = find_missing_sprites(project_root);
    if missing.is_empty() {
        println!("All sprites are present!");
        return;
    }
    println!("Missing sprites ({}):", missing.len());
    for spec in &missing {
        println!(
            "  {} ({:?}) -> {}",
            spec.name,
            spec.category,
            spec.asset_path()
        );
    }
}

fn cmd_preview(name: &str) -> Result<(), SpriteGenError> {
    let spec = find_sprite(name).ok_or_else(|| SpriteGenError::NotFound(name.to_string()))?;
    let prompt = build_prompt(spec);
    println!("=== Prompt for '{}' ===", spec.name);
    println!("{}", prompt);
    println!("=== Output path ===");
    println!("{}", spec.asset_path());
    Ok(())
}

async fn cmd_generate(
    provider_choice: &ProviderChoice,
    project_root: &Path,
    name: &str,
) -> Result<(), SpriteGenError> {
    let spec = find_sprite(name).ok_or_else(|| SpriteGenError::NotFound(name.to_string()))?;
    let prompt = build_prompt(spec);

    println!("Generating sprite: {} ...", spec.name);
    tracing::info!(sprite = spec.name, "generating sprite");

    let provider = create_provider(provider_choice)?;
    let raw_image = provider.generate(&prompt).await?;
    let processed = postprocess_image(&raw_image)?;

    let output_path = spec.full_path(project_root);
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    processed
        .save(&output_path)
        .map_err(|e| SpriteGenError::ImageProcessing(format!("failed to save PNG: {}", e)))?;

    println!("Saved: {}", output_path.display());
    Ok(())
}

async fn cmd_batch(
    provider_choice: &ProviderChoice,
    project_root: &Path,
) -> Result<(), SpriteGenError> {
    let missing = find_missing_sprites(project_root);
    if missing.is_empty() {
        println!("All sprites are present! Nothing to generate.");
        return Ok(());
    }

    println!("Generating {} missing sprites...", missing.len());

    let provider = create_provider(provider_choice)?;
    let pb = indicatif::ProgressBar::new(missing.len() as u64);
    pb.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} {msg}")
            .expect("invalid progress bar template"),
    );

    for spec in &missing {
        pb.set_message(spec.name.to_string());
        let prompt = build_prompt(spec);

        match provider.generate(&prompt).await {
            Ok(raw_image) => match postprocess_image(&raw_image) {
                Ok(processed) => {
                    let output_path = spec.full_path(project_root);
                    if let Some(parent) = output_path.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    processed.save(&output_path).map_err(|e| {
                        SpriteGenError::ImageProcessing(format!("failed to save PNG: {}", e))
                    })?;
                    tracing::info!(sprite = spec.name, "saved");
                }
                Err(e) => {
                    pb.println(format!("  [WARN] {} postprocess failed: {}", spec.name, e));
                }
            },
            Err(e) => {
                pb.println(format!("  [WARN] {} generation failed: {}", spec.name, e));
            }
        }

        pb.inc(1);
    }

    pb.finish_with_message("done");
    Ok(())
}

fn create_provider(choice: &ProviderChoice) -> Result<Provider, SpriteGenError> {
    match choice {
        ProviderChoice::Openrouter => Provider::openrouter(),
        ProviderChoice::Openai => Provider::openai(),
    }
}
