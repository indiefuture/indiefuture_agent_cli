# IndieFuture Agent CLI - Design Specification

## Overview

The IndieFuture Agent CLI is a Rust-based AI agent designed to:
1. Listen for user commands via a CLI interface built with cliclack
2. Break down complex tasks into manageable subtasks
3. Execute those subtasks sequentially or in parallel
4. Use local codebase analysis and memory management to provide context-rich interactions with AI models

## Core Architecture

### 1. Command Interface Layer
- Uses cliclack for interactive CLI experience
- Command parsing and validation
- Task submission handling
- Progress feedback and output formatting

### 2. Task Management Layer
- Task decomposition engine
- Subtask generation and prioritization
- Execution orchestration
- Dependency management between subtasks

### 3. Memory Management Layer
- **Short-term Memory**: Current conversation and immediate context
- **Long-term Memory**: Vector embeddings for semantic search
- **Tool State Memory**: Persistent state for tools and their operations

### 4. AI Communication Layer
- OpenAI API integration
- Claude API integration
- Prompt construction with context enrichment
- Response parsing and handling

### 5. Code Analysis Layer
- Local codebase scanning
- Code parsing and understanding
- Embedding generation from code
- Contextual relevance matching

## Detailed Components

### Vector Storage (Qdrant)
- **Purpose**: Store and retrieve code snippets and context by semantic similarity
- **Implementation**: Use Qdrant's Rust client
- **Key Functions**:
  - Store code embeddings
  - Perform similarity searches
  - Maintain collection structure
  - Handle embedding versioning

### Persistence Layer (Sled)
- **Purpose**: Maintain context between CLI sessions
- **Implementation**: Sled for key-value storage
- **Key Functions**:
  - Store conversation history
  - Maintain tool states
  - Cache frequent queries
  - Persist user preferences

### Task Decomposition Engine
- **Purpose**: Break down complex tasks into manageable subtasks
- **Implementation**: AI-assisted decomposition with templates
- **Key Functions**:
  - Analyze task complexity
  - Generate dependency graph of subtasks
  - Prioritize subtask execution
  - Track subtask completion

### Codebase Scanner
- **Purpose**: Scan, parse, and understand local codebase
- **Implementation**: File system traversal with parsing
- **Key Functions**:
  - Walk directory structure
  - Parse code files of different languages
  - Extract semantic information
  - Generate embeddings from code

### AI Integration
- **Purpose**: Leverage AI models for task execution
- **Implementation**: API clients for OpenAI and Claude
- **Key Functions**:
  - Construct context-rich prompts
  - Manage API request limits and retries
  - Parse and validate AI responses
  - Handle streaming responses

## Data Flow

1. **Command Input**:
   - User enters command via cliclack interface
   - Command is parsed and validated

2. **Task Analysis**:
   - Command is sent to task decomposition engine
   - AI model helps break down task into subtasks
   - Dependency graph and execution plan created

3. **Context Gathering**:
   - Codebase scanner identifies relevant files
   - Files are parsed and embedded
   - Vector store is queried for similar contexts
   - Short-term memory is updated with current task context

4. **Subtask Execution**:
   - Subtasks executed according to dependency graph
   - Each subtask enriched with relevant context
   - Results stored in short-term memory
   - Progress reported to user

5. **AI Interaction**:
   - Rich context sent to AI model (OpenAI/Claude)
   - Responses parsed and validated
   - Results used in subsequent subtasks
   - User updated on progress

6. **Result Consolidation**:
   - Subtask results combined
   - Final output formatted for user
   - Long-term memory updated with new learnings
   - Tool states persisted for future use

## Implementation Plan

### Phase 1: Core Infrastructure
- Setup project structure
- Implement basic CLI with cliclack
- Setup Qdrant and Sled clients
- Build file system scanner

### Phase 2: Memory Management
- Implement embedding generation
- Setup vector storage collections
- Build context management system
- Create persistence layer

### Phase 3: Task Management
- Design task representation format
- Implement task decomposition logic
- Build execution orchestration
- Create dependency resolver

### Phase 4: AI Integration
- Setup OpenAI API client
- Setup Claude API client
- Build prompt enrichment system
- Implement response handlers

### Phase 5: Command Interface
- Enhance CLI experience
- Add progress visualization
- Implement error handling
- Add configuration management

## Technology Stack

### Core Dependencies
- **Rust**: Base programming language
- **Tokio**: Async runtime
- **Serde**: Serialization/deserialization
- **Cliclack**: Interactive CLI interface
- **Reqwest**: HTTP client for API calls

### Storage
- **Qdrant**: Vector database for embeddings
- **Sled**: Embedded database for persistence

### AI Integration
- **OpenAI API Client**: For GPT model access
- **Claude API Client**: For Claude model access

### Code Analysis
- **Tree-sitter**: (Optional) For advanced code parsing
- **Walkdir**: For file system traversal

## Error Handling Strategy
- Comprehensive error types using thiserror
- Graceful degradation when services are unavailable
- Detailed logging for debugging
- User-friendly error messages

## Configuration Management
- Environment-based configuration via dotenvy
- User preference storage in Sled
- Command-line overrides
- Sensible defaults

## Security Considerations
- API key management
- Local-only data storage
- No telemetry without explicit consent
- Proper error handling to prevent information leakage

## Testing Strategy
- Unit tests for core components
- Integration tests for service interactions
- Property-based testing for complex logic
- Manual testing for CLI experience

## Future Expansion
- Plugin system for adding new tools
- Remote agent integration
- Web interface option
- Team collaboration features