# iMi - Gemini Context

This document provides a comprehensive overview of the `iMi` project, designed to be used as a context file for Gemini.

## Project Overview

`iMi` is a command-line tool written in Rust for managing Git worktrees. It is specifically designed to support decentralized, multi-agent development workflows. The tool provides a structured and opinionated way to create and manage worktrees for different types of tasks, such as feature development, bug fixes, and code reviews.

### Key Technologies

*   **Rust:** The core language for the project.
*   **Tokio:** Used for asynchronous operations.
*   **Clap:** For parsing command-line arguments.
*   **Git2:** For Git operations.
*   **SQLx:** For database interactions (SQLite).
*   **Serde:** For serialization and deserialization.
*   **Toml:** For configuration file parsing.

### Architecture

The project is structured into several modules, each with a specific responsibility:

*   `main.rs`: The application's entry point.
*   `cli.rs`: Defines the command-line interface.
*   `config.rs`: Manages application configuration.
*   `database.rs`: Handles database interactions.
*   `error.rs`: Defines custom error types.
*   `git.rs`: Contains Git-related functionality.
*   `init.rs`: Handles project initialization.
*   `monitor.rs`: Implements real-time monitoring.
*   `worktree.rs`: Manages Git worktrees.

## Building and Running

### Building

To build the project, use the following command:

```bash
cargo build
```

For a release build, use:

```bash
cargo build --release
```

### Running

Once built, the tool can be run directly using `cargo run` or by installing it and running the `iMi` executable.

```bash
cargo run -- <command>
```

### Testing

To run the test suite, use:

```bash
cargo test
```

## Development Conventions

*   **Asynchronous Code:** The project uses `async/await` extensively with the Tokio runtime.
*   **Error Handling:** The `anyhow` and `thiserror` crates are used for robust error handling.
*   **Modularity:** The code is organized into small, focused modules.
*   **Conventional Commits:** The project follows the Conventional Commits specification for commit messages.

## Commands

The following commands are available:

| Command | Description |
|---|---|
| `feat <name>` | Create a new feature worktree. |
| `review <pr_number>` | Create a worktree for reviewing a pull request. |
| `fix <name>` | Create a worktree for bug fixes. |
| `aiops <name>` | Create a worktree for AI operations. |
| `devops <name>` | Create a worktree for DevOps tasks. |
| `trunk` | Switch to the trunk worktree. |
| `status` | Show the status of all worktrees. |
| `list` | List all active worktrees. |
| `remove <name>` | Remove a worktree. |
| `monitor` | Start real-time monitoring of worktree activities. |
| `init` | Initialize `iMi` in the current directory. |

## Directory Structure

The project follows a standard Rust project layout:

```
.
├── Cargo.toml
├── src
│   ├── main.rs
│   ├── lib.rs
│   ├── cli.rs
│   ├── config.rs
│   ├── database.rs
│   ├── error.rs
│   ├── git.rs
│   ├── init.rs
│   ├── monitor.rs
│   └── worktree.rs
└── tests
    ├── ...
```

## Configuration

`iMi` can be configured via a `config.toml` file located in `~/.config/iMi/`. The configuration allows for customization of settings related to synchronization, Git, and monitoring.

## Agent Integration

`iMi` is designed to facilitate multi-agent workflows by:

*   **Providing separate worktrees:** This prevents conflicts between agents working on different tasks.
*   **Tracking agent activity:** All file changes are logged with timestamps, allowing for monitoring of agent activities.
*   **Real-time visibility:** The `monitor` command provides a real-time view of all agent activities.
