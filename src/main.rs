use indiefuture_cli::agent_engine::{AgentEngine, SharedState};
use indiefuture_cli::memory::ContextMemory;
use indiefuture_cli::{ai::create_ai_client, config::Settings, error::AgentResult, run_cli};
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> AgentResult<()> {
    // Initialize logging
    env_logger::init();

    // Load settings
    let settings = Settings::load()?;

    let ai_client = create_ai_client(
        &settings.default_ai_provider,
        &settings.default_model,
        settings.openai_api_key.as_deref().unwrap_or(""),
    )?;

    let shared_state = SharedState { ai_client };

    let context_memory = Mutex::new(ContextMemory::default());
    let agent_engine = Mutex::new(AgentEngine::default());

    // Run CLI interface
    run_cli(
        Arc::new(shared_state), //contains ai data
        Arc::new(context_memory),
        Arc::new(settings),
        Arc::new(agent_engine),
    )
    .await
}
