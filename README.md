# oma (Ollama Messaging Assistant)

A minimal TUI wrapper around Ollama-based chat LLMs.

## Prerequisites

- **Ollama** installed and running locally.
  Download & installation instructions: https://ollama.com
- At least one Ollama model pulled (e.g. `ollama pull deepseek-r1:1.5b`).

## Features

- Streamed responses from local Ollama API
- Simple terminal UI with history, scrolling and spinner
- Configurable model and endpoint via `~/.config/oma/config.toml`

## Installation

```bash
git clone https://github.com/casonadams/ollama-rs.git
cd oma
cargo install --path .
```

## Configuration

On first run, a default config is written to:

```
~/.config/oma/config.toml
```

```toml
model = "deepseek-r1:1.5b"
uri   = "http://localhost:11434"
# Optional system prompt:
system = """You are a helpful assistant."""
```

## Usage

```bash
oma
    Type your prompt, press Enter
    Ctrl+C to exit
    ↑/↓/PageUp/PageDown to scroll
```
