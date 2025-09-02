# iMi - Git Worktree Management Tool

iMi is a sophisticated Rust-based Git worktree management tool designed for asynchronous, parallel multi-agent workflows. It's a key component of the [33GOD](https://github.com/33god/33god) Agentic Software Pipeline.

## 🚀 Features

- **Intelligent Worktree Management**: Create and manage Git worktrees with opinionated conventions
- **Real-time Monitoring**: Track file changes and agent activities across all worktrees
- **Database Tracking**: SQLite-based persistence for worktree history and agent coordination
- **Symlink Management**: Automatic dotfile synchronization across worktrees
- **33GOD Integration**: Built-in support for multi-agent coordination
- **Convention over Configuration**: Minimal setup, maximum productivity

## 📦 Installation

### From Source

```bash
git clone https://github.com/33god/imi.git
cd imi
cargo install --path .
```

### Using Cargo

```bash
cargo install imi
```

## 🎯 Quick Start

### Initialize a Repository Structure

```bash
# Create a feature worktree
imi feat user-authentication

# Review a pull request
imi review 42

# Create a bug fix worktree
imi fix critical-security-issue

# DevOps tasks
imi devops ci-pipeline-update

# AI operations (agents, rules, workflows)
imi aiops new-agent-workflow
```

### Monitor Activity

```bash
# Real-time monitoring of all worktrees
imi monitor

# Show status of all worktrees
imi status

# List active worktrees
imi list
```

## 📋 Commands

| Command | Description | Example |
|---------|-------------|---------|
| `imi feat <name>` | Create feature worktree | `imi feat user-login` |
| `imi review <pr>` | Create PR review worktree | `imi review 123` |
| `imi fix <name>` | Create bugfix worktree | `imi fix auth-bug` |
| `imi aiops <name>` | Create AI operations worktree | `imi aiops agent-config` |
| `imi devops <name>` | Create DevOps worktree | `imi devops ci-update` |
| `imi trunk` | Switch to trunk worktree | `imi trunk` |
| `imi status` | Show worktree status | `imi status` |
| `imi list` | List all worktrees | `imi list` |
| `imi remove <name>` | Remove a worktree | `imi remove feat-old` |
| `imi monitor` | Start real-time monitoring | `imi monitor` |

## 🏗️ Directory Structure

iMi follows a standardized directory convention:

```
~/code/my-project/
├── trunk-main/              # Main branch worktree
├── feat-user-auth/          # Feature worktree
├── pr-123/                  # Pull request review worktree  
├── fix-security-bug/        # Bug fix worktree
├── aiops-new-agent/         # AI operations worktree
├── devops-ci-update/        # DevOps worktree
└── sync/                    # Shared configuration
    ├── global/              # Global sync files
    │   ├── coding-rules.md
    │   └── stack-specific.md
    └── repo/                # Repository-specific sync
        ├── .env
        ├── .jarad-config
        └── .vscode/
```

## ⚙️ Configuration

iMi uses convention over configuration but allows customization via `~/.config/imi/config.toml`:

```toml
[sync_settings]
enabled = true
global_sync_path = "sync/global" 
repo_sync_path = "sync/repo"

[git_settings]
default_branch = "main"
remote_name = "origin"
auto_fetch = true
prune_on_fetch = true

[monitoring_settings]
enabled = true
refresh_interval_ms = 1000
watch_file_changes = true
track_agent_activity = true

# Files to symlink across worktrees
symlink_files = [
    ".env",
    ".jarad-config", 
    ".vscode/settings.json",
    ".gitignore.local"
]
```

## 🤖 Agent Integration

iMi is designed for multi-agent workflows:

- **Activity Tracking**: All file changes are logged with timestamps
- **Agent Identification**: Track which agents are working in which worktrees
- **Conflict Prevention**: Separate worktrees prevent merge conflicts
- **Real-time Visibility**: Monitor all agent activities in real-time

### Agent Coordination Example

```bash
# Agent 1: Feature development
imi feat payment-integration
# -> Creates feat-payment-integration/ worktree

# Agent 2: Code review
imi review 456  
# -> Creates pr-456/ worktree

# Agent 3: Bug fix
imi fix payment-bug
# -> Creates fix-payment-bug/ worktree

# Monitor all activities
imi monitor
# -> Real-time view of all agents
```

## 📊 Monitoring & Analytics

### Real-time Monitoring

```bash
imi monitor
```

Shows:
- File changes across all worktrees
- Git status (commits ahead/behind, dirty files)
- Agent activities and timestamps
- Performance metrics

### Status Dashboard

```bash
imi status
```

Displays:
- Active worktrees by type
- Git status for each worktree
- Recent agent activities
- Branch synchronization status

## 🔧 Troubleshooting

### Common Issues

**Worktree not found**
```bash
# Ensure you're in a Git repository
git status
# Or specify the repository explicitly
imi --repo my-project feat new-feature
```

**Permission errors**
```bash
# Check directory permissions
ls -la ~/.config/imi/
# Reset configuration
rm ~/.config/imi/config.toml
```

**Database corruption**
```bash
# Reset the database
rm ~/.config/imi/imi.db
```

## 🤝 Contributing

1. Fork the repository
2. Create your feature branch: `imi feat my-new-feature`
3. Commit your changes: `git commit -am 'Add some feature'`
4. Push to the branch: `git push origin feat/my-new-feature`
5. Submit a pull request

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🔗 Related Projects

- [33GOD](https://github.com/33god/33god) - The complete agentic software pipeline
- [Claude Flow](https://github.com/ruvnet/claude-flow) - Multi-agent coordination framework

## 📞 Support

- Issues: [GitHub Issues](https://github.com/33god/imi/issues)
- Documentation: [GitHub Wiki](https://github.com/33god/imi/wiki)
- Discord: [33GOD Community](https://discord.gg/33god)

---

Built with ❤️ for the 33GOD Agentic Software Pipeline