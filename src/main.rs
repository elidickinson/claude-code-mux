use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod cli;
mod models;
mod pid;
mod providers;
mod router;
mod server;

#[derive(Parser)]
#[command(name = "ccm")]
#[command(about = "Claude Code Mux - High-performance router built in Rust", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Path to configuration file
    #[arg(short, long, default_value = "config/default.toml")]
    config: PathBuf,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the router service
    Start {
        /// Port to listen on
        #[arg(short, long)]
        port: Option<u16>,
    },
    /// Stop the router service
    Stop,
    /// Restart the router service
    Restart,
    /// Check service status
    Status,
    /// Initialize configuration interactively
    Init,
    /// Manage models and providers
    Model,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    // Load configuration
    let config = cli::AppConfig::from_file(&cli.config)?;

    match cli.command {
        Commands::Start { port } => {
            let mut config = config;

            // Override port if specified
            if let Some(port) = port {
                config.server.port = port;
            }

            // Write PID file
            if let Err(e) = pid::write_pid() {
                eprintln!("Warning: Failed to write PID file: {}", e);
            }

            tracing::info!("Starting Claude Code Mux on port {}", config.server.port);
            println!("🚀 Claude Code Mux v{}", env!("CARGO_PKG_VERSION"));
            println!("📡 Starting server on {}:{}", config.server.host, config.server.port);
            println!();
            println!("⚡️ Rust-powered for maximum performance");
            println!("🧠 Intelligent context-aware routing");
            println!();

            // Display routing configuration
            println!("🔀 Router Configuration:");
            println!("   Default: {}", config.router.default);
            if let Some(ref bg) = config.router.background {
                println!("   Background: {}", bg);
            }
            if let Some(ref think) = config.router.think {
                println!("   Think: {}", think);
            }
            if let Some(ref ws) = config.router.websearch {
                println!("   WebSearch: {}", ws);
            }
            println!();
            println!("Press Ctrl+C to stop");

            // Cleanup PID file on exit
            let result = server::start_server(config).await;
            let _ = pid::cleanup_pid();
            result?;
        }
        Commands::Stop => {
            println!("Stopping Claude Code Mux...");
            match pid::read_pid() {
                Ok(pid) => {
                    if pid::is_process_running(pid) {
                        #[cfg(unix)]
                        {
                            use nix::sys::signal::{kill, Signal};
                            use nix::unistd::Pid;

                            if let Err(e) = kill(Pid::from_raw(pid as i32), Signal::SIGTERM) {
                                eprintln!("Failed to stop service: {}", e);
                            } else {
                                println!("✅ Service stopped successfully");
                                let _ = pid::cleanup_pid();
                            }
                        }
                        #[cfg(windows)]
                        {
                            use std::process::Command;
                            let _ = Command::new("taskkill")
                                .args(&["/PID", &pid.to_string(), "/F"])
                                .output();
                            println!("✅ Service stopped successfully");
                            let _ = pid::cleanup_pid();
                        }
                    } else {
                        println!("Service is not running");
                        let _ = pid::cleanup_pid();
                    }
                }
                Err(_) => {
                    println!("Service is not running (no PID file found)");
                }
            }
        }
        Commands::Restart => {
            println!("Restarting Claude Code Mux...");

            // Stop the existing service
            match pid::read_pid() {
                Ok(pid) => {
                    if pid::is_process_running(pid) {
                        println!("Stopping existing service...");
                        #[cfg(unix)]
                        {
                            use nix::sys::signal::{kill, Signal};
                            use nix::unistd::Pid;

                            let _ = kill(Pid::from_raw(pid as i32), Signal::SIGTERM);
                        }
                        #[cfg(windows)]
                        {
                            use std::process::Command;
                            let _ = Command::new("taskkill")
                                .args(&["/PID", &pid.to_string(), "/F"])
                                .output();
                        }
                        // Wait a bit for the process to exit
                        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                    }
                }
                Err(_) => {
                    println!("No existing service found");
                }
            }
            let _ = pid::cleanup_pid();

            // Start the service in the background
            println!("Starting service...");
            use std::process::Command;

            let exe_path = std::env::current_exe()?;
            let mut cmd = Command::new(&exe_path);
            cmd.arg("start");

            // Pass the config file if it was specified
            if cli.config != PathBuf::from("config/default.toml") {
                cmd.arg("--config").arg(cli.config);
            }

            // Spawn detached process
            #[cfg(unix)]
            {
                use std::os::unix::process::CommandExt;
                unsafe {
                    cmd.pre_exec(|| {
                        // Create a new process group
                        nix::libc::setsid();
                        Ok(())
                    });
                }
            }

            cmd.stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null());

            match cmd.spawn() {
                Ok(_) => {
                    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
                    println!("✅ Service restarted successfully");
                }
                Err(e) => {
                    eprintln!("Failed to restart service: {}", e);
                }
            }
        }
        Commands::Status => {
            println!("Checking service status...");
            match pid::read_pid() {
                Ok(pid) => {
                    if pid::is_process_running(pid) {
                        println!("✅ Service is running (PID: {})", pid);
                    } else {
                        println!("❌ Service is not running (stale PID file)");
                        let _ = pid::cleanup_pid();
                    }
                }
                Err(_) => {
                    println!("❌ Service is not running");
                }
            }
        }
        Commands::Init => {
            println!("🔧 Interactive Configuration Setup");
            println!();
            println!("This feature will guide you through setting up your configuration.");
            println!("For now, please edit config/default.toml manually.");
            // TODO: Implement interactive setup with prompts
        }
        Commands::Model => {
            println!("📊 Model Configuration");
            println!();
            println!("Configured Models:");
            println!("  • Default: {}", config.router.default);
            if let Some(ref think) = config.router.think {
                println!("  • Think: {}", think);
            }
            if let Some(ref ws) = config.router.websearch {
                println!("  • WebSearch: {}", ws);
            }
            if let Some(ref bg) = config.router.background {
                println!("  • Background: {}", bg);
            }
            println!();
            println!("Providers:");
            for provider in &config.providers {
                if provider.enabled.unwrap_or(false) {
                    println!("  • {} ({})", provider.name, provider.provider_type);
                }
            }
        }
    }

    Ok(())
}
