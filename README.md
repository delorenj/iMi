# iMi - Git Worktree Management Tool

iMi is a sophisticated Rust-based Git worktree management tool designed for asynchronous, parallel multi-agent workflows. It's a key component of the [33GOD](https://github.com/delorenj/33god) Agentic Software Pipeline.

## 🚀 Features

- **Intelligent Worktree Management**: Create and manage Git worktrees with opinionated conventions
- **Real-time Monitoring**: Track file changes and agent activities across all worktrees
- **Database Tracking**: PostgreSQL-based registry for project and worktree tracking across the 33GOD ecosystem
- **Symlink Management**: Automatic dotfile synchronization across worktrees
- **33GOD Integration**: Built-in support for multi-agent coordination
- **Convention over Configuration**: Minimal setup, maximum productivity

## 📦 Installation

### From Source

```bash
git clone https://github.com/delorenj/iMi
cd iMi
cargo install --path .
```

### Using Cargo

```bash
cargo install iMi
```

## 🎯 Quick Start

### Initialize a Repository Structure

```bash
# Create a feature worktree
iMi feat user-authentication

# Review a pull request
iMi review 42

# Create a bug fix worktree
iMi fix critical-security-issue

# DevOps tasks
iMi devops ci-pipeline-update

# AI operations (agents, rules, workflows)
iMi aiops new-agent-workflow
```

### Monitor Activity

```bash
# Real-time monitoring of all worktrees
iMi monitor

# Show status of all worktrees
iMi status

# List active worktrees
iMi list
```

## 📋 Commands

| Command | Description | Example |
|---------|-------------|---------|
| `iMi feat <name>` | Create feature worktree | `iMi feat user-login` |
| `iMi review <pr>` | Create PR review worktree | `iMi review 123` |
| `iMi fix <name>` | Create bugfix worktree | `iMi fix auth-bug` |
| `iMi aiops <name>` | Create AI operations worktree | `iMi aiops agent-config` |
| `iMi devops <name>` | Create DevOps worktree | `iMi devops ci-update` |
| `iMi trunk` | Switch to trunk worktree | `iMi trunk` |
| `iMi status` | Show worktree status | `iMi status` |
| `iMi list` | List all worktrees | `iMi list` |
| `iMi remove <name>` | Remove a worktree | `iMi remove feat-old` |
| `iMi monitor` | Start real-time monitoring | `iMi monitor` |

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
        └── .vscode/
```

## ⚙️ Configuration

iMi uses convention over configuration but allows customization via `~/.config/iMi/config.toml`:

```toml
[sync_settings]
enabled = true
user_sync_path = "sync/user"
local_sync_path = "sync/local"

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
iMi feat payment-integration
# -> Creates feat-payment-integration/ worktree

# Agent 2: Code review
iMi review 456  
# -> Creates pr-456/ worktree

# Agent 3: Bug fix
iMi fix payment-bug
# -> Creates fix-payment-bug/ worktree

# Monitor all activities
iMi monitor
# -> Real-time view of all agents
```

## 📊 Monitoring & Analytics

### Real-time Monitoring

```bash
iMi monitor
```

Shows:
- File changes across all worktrees
- Git status (commits ahead/behind, dirty files)
- Agent activities and timestamps
- Performance metrics

### Status Dashboard

```bash
iMi status
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
iMi --repo my-project feat new-feature
```

**Permission errors**
```bash
# Check directory permissions
ls -la ~/.config/iMi/
# Reset configuration
rm ~/.config/iMi/config.toml
```

**Database corruption**
```bash
# Reset the database
rm ~/.config/iMi/iMi.db
```

## 🤝 Contributing

1. Fork the repository
2. Create your feature branch: `iMi feat my-new-feature`
3. Commit your changes: `git commit -am 'Add some feature'`
4. Push to the branch: `git push origin feat/my-new-feature`
5. Submit a pull request

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🔗 Related Projects

- [33GOD](https://github.com/delorenj/33GOD)

## 📞 Support

- Issues: [GitHub Issues](https://github.com/33god/iMi/issues)
- Documentation: [GitHub Wiki](https://github.com/33god/iMi/wiki)
- Discord: [33GOD Community](https://discord.gg/33god)

---

Built with ❤️
