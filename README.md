# Indiefuture Agent CLI 

An AI-powered CLI agent that can listen for commands, break them down into subtasks, and execute them with context from your local codebase.




### TRY ME 

```

start this bot with 'cargo r'  


Tell this bot  to add 'hello world'  to the end of the readme of its own project 


it will :D  


```









## Features

- Interactive CLI interface using cliclack
- Task decomposition into manageable subtasks
- Parallel task execution with dependency management
- Code scanning and analysis for context-enriched AI queries
- Memory management with both short-term and long-term storage
- Integration with OpenAI and Claude APIs
- Vector storage for semantic code search

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

- "Analyze this codebase and create a UML diagram"
- "Find all TODO comments and create GitHub issues for them"
- "Refactor the error handling in the src/utils directory"
- "Write unit tests for the Parser class"
 
## License

MIT
