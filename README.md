# chat-echo

[![CI](https://github.com/dnacenta/chat-echo/actions/workflows/ci.yml/badge.svg?branch=development)](https://github.com/dnacenta/chat-echo/actions/workflows/ci.yml)
[![License: GPL-3.0](https://img.shields.io/github/license/dnacenta/chat-echo)](LICENSE)
[![Version](https://img.shields.io/github/v/tag/dnacenta/chat-echo?label=version&color=green)](https://github.com/dnacenta/chat-echo/tags)
[![Rust](https://img.shields.io/badge/rust-1.80%2B-orange)](https://rustup.rs/)

Web chat interface for the Echo ecosystem. The default communication channel — open a browser and talk to your AI entity.

## Why

Every AI entity needs at least one way to communicate. chat-echo is the zero-friction option: no Twilio account, no Discord bot, no setup beyond running the binary. Open a browser, start chatting.

## How It Works

```
┌──────────────────────────┐
│    Browser (Frontend)     │
│  HTML + CSS + vanilla JS  │
│  WebSocket connection     │
└────────────┬─────────────┘
             │ ws://host:port/ws
┌────────────┴─────────────┐
│   chat-echo (Rust/axum)   │
│  Static file server       │
│  WebSocket ↔ HTTP relay   │
└────────────┬─────────────┘
             │ POST /chat
┌────────────┴─────────────┐
│      bridge-echo          │
│  LLM session management   │
│  Security + identity      │
└──────────────────────────┘
```

chat-echo is a thin transport layer. It serves the web UI, relays messages to [bridge-echo](https://github.com/dnacenta/bridge-echo) over HTTP, and streams responses back to the browser over WebSocket. All LLM logic, session management, and security live in bridge-echo.

## Installation

### From source

```bash
git clone https://github.com/dnacenta/chat-echo.git
cd chat-echo
cargo install --path .
```

## Configuration

All configuration via environment variables:

| Variable | Default | Description |
|----------|---------|-------------|
| `CHAT_ECHO_HOST` | `0.0.0.0` | Bind address |
| `CHAT_ECHO_PORT` | `8080` | HTTP port |
| `CHAT_ECHO_BRIDGE_URL` | `http://127.0.0.1:3100` | bridge-echo endpoint |
| `CHAT_ECHO_STATIC_DIR` | `./static` | Path to frontend files |

## Usage

```bash
# Start with defaults (assumes bridge-echo on localhost:3100)
chat-echo

# Custom port and bridge URL
CHAT_ECHO_PORT=9000 CHAT_ECHO_BRIDGE_URL=http://my-bridge:3100 chat-echo
```

Then open `http://localhost:8080` in your browser.

## Requirements

- [bridge-echo](https://github.com/dnacenta/bridge-echo) running and accessible
- Rust 1.80+ (for building from source)

## Ecosystem

chat-echo is part of the echo ecosystem — a set of composable tools for building autonomous AI entities:

| Module | Purpose | Repo |
|--------|---------|------|
| [recall-echo](https://github.com/dnacenta/recall-echo) | Persistent three-layer memory | [![crates.io](https://img.shields.io/crates/v/recall-echo)](https://crates.io/crates/recall-echo) |
| [praxis-echo](https://github.com/dnacenta/praxis-echo) | Document pipeline enforcement | [source](https://github.com/dnacenta/praxis-echo) |
| [vigil-echo](https://github.com/dnacenta/vigil-echo) | Metacognitive monitoring | [source](https://github.com/dnacenta/vigil-echo) |
| [voice-echo](https://github.com/dnacenta/voice-echo) | Voice interface (phone calls) | [source](https://github.com/dnacenta/voice-echo) |
| [bridge-echo](https://github.com/dnacenta/bridge-echo) | HTTP bridge for Claude CLI | [![crates.io](https://img.shields.io/crates/v/bridge-echo)](https://crates.io/crates/bridge-echo) |
| [discord-voice-echo](https://github.com/dnacenta/discord-voice-echo) | Discord voice channel sidecar | [source](https://github.com/dnacenta/discord-voice-echo) |
| **chat-echo** | **Web chat interface** | **you are here** |

## License

[GPL-3.0](LICENSE)
