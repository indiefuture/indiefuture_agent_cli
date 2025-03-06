use indiefuture_cli::{config::Settings, error::AgentResult, run_cli};
use std::sync::Arc;

#[tokio::main]
async fn main() -> AgentResult<()> {
    // Initialize logging
    env_logger::init();
    
    // Load settings
    let settings = Settings::load()?;
    
    // Run CLI interface
    run_cli(Arc::new(settings)).await
}