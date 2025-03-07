# Indiefuture Agent CLI 

An AI-powered CLI agent that can listen for commands, break them down into subtasks, and execute them with context from your local codebase.

Requires an openAI API key in ENV.   


### TRY ME 

```

start this bot with 'cargo r'  


Tell this bot  to add 'hello world'  to the end of the readme of its own project 


it will :D  


```



CHAT 

https://discord.gg/nhS9hkJM4z





## Features

- Interactive CLI interface using cliclack
- Task decomposition into manageable subtasks
- Sequential task execution with dependency management 
- Integration with OpenAI and Claude APIs
- Simple memory storage for semantic code search (prob can be improved ! ) 

## Requirements

- Rust 1.80+ 

## Installation

```bash
# Clone the repository
git clone https://github.com/your-username/indiefuture_agent_cli.git
cd indiefuture_agent_cli

# Build the project
cargo build --release

# Run the agent
cargo run --release
```

## Configuration

Create a `.env` file in the project root with the following variables:

```
OPENAI_API_KEY=your_openai_api_key
CLAUDE_API_KEY=your_claude_api_key
DEFAULT_AI_PROVIDER=openai
DEFAULT_MODEL=gpt-4o
 
```

## Usage

```bash
# Start the interactive CLI
cargo run  
 
```

## Example Tasks

- "Tell me about this project"
 
 
## License

MIT
