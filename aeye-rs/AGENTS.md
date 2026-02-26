# A-Eye Development Guide

## Overview

This is the A-Eye CLI, an open source agentic coding interface.

## Project Structure

```
aeye-rs/
├── core/           # Core functionality - config, policy, models
├── tui/            # Terminal UI
└── Cargo.toml      # Workspace configuration
```

## Building

```bash
cargo build -p aeye-core
cargo build -p aeye-tui
```

## Testing

```bash
cargo test -p aeye-core
```

## Code Style

- Follow existing code conventions
- Use meaningful variable names
- Add tests for new functionality
