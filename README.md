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
| `iMi add <type> <name>` | Create typed worktree (preferred) | `iMi add feat user-login` |
| `iMi feat <name>` | Create feature worktree | `iMi feat user-login` |
| `iMi review <pr>` | Create PR review worktree | `iMi review 123` |
| `iMi fix <name>` | Create bugfix worktree | `iMi fix auth-bug` |
| `iMi aiops <name>` | Create AI operations worktree | `iMi aiops agent-config` |
| `iMi devops <name>` | Create DevOps worktree | `iMi devops ci-update` |
| `iMi trunk` | Switch to trunk worktree | `iMi trunk` |
| `iMi status` | Show worktree status | `iMi status` |
| `iMi list` | List all worktrees | `iMi list` |
| `iMi remove <name>` | Remove a worktree | `iMi remove feat-old` |
| `iMi metadata set ...` | Set worktree metadata key/value | `iMi metadata set --worktree feat-auth --key plane.ticket_id --value PROJ-123` |
| `iMi metadata get ...` | Read worktree metadata | `iMi metadata get --worktree feat-auth --key plane.ticket_id` |
| `iMi migrate-office` | Migrate registered repos into office layout | `iMi migrate-office --dry-run` |
| `iMi monitor` | Start real-time monitoring | `iMi monitor` |

## 🏗️ Workspace Structure

iMi uses entity-based workspace isolation for true multi-agent collaboration:

```
~/33GOD/workspaces/
├── delorenj/                # Your workspace
│   ├── my-project/          # Full clone of project
│   │   ├── trunk-main/      # Main branch worktree
│   │   ├── feat-auth/       # Feature worktree
│   │   └── fix-bug/         # Bug fix worktree
│   └── other-project/
└── yi-backend-001/          # Yi agent workspace (when implemented)
    └── my-project/
        └── feat-api/

# Each entity has complete isolation
# Cross-entity access requires explicit ticket reference
```

## 🏢 Office Layout Rules

Worktrees are now enforced to live in each entity's dedicated office clone layout:

```bash
${IMI_WORKSPACE_ROOT:-~/33GOD/workspaces}/${IMI_ENTITY_ID:-$USER}/<repo>/
├── trunk-main/
├── feat-*
├── fix-*
├── aiops-*
└── devops-*
```

Rules:
- No shared master clone across entities.
- Every entity has its own full repo clone (`trunk-*`) and sibling worktrees.
- `iMi init` migrates non-office repos into this layout before registration.
- Worktree operations are rejected when a repository is outside the current entity office.

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

[workspace_settings]
root_path = "/home/you/33GOD/workspaces"
entity_id = "delorenj"

# Files to symlink across worktrees
symlink_files = [
    ".env",
    ".vscode/settings.json",
    ".gitignore.local"
]
```

## 🤖 Agent Integration

iMi treats all actors (humans and Yi agents) as equal **entities** with token-based authentication:

- **Entity-based Authentication**: All iMi commands require `$IMI_IDENTITY_TOKEN`
- **Workspace Isolation**: Each entity has a completely isolated workspace
- **Cross-entity Accountability**: All workspace access is logged with optional ticket reference
- **Yi Integration Ready**: Flume will provision Yi agents with tokens (not yet implemented)

### Entity-Based Workflow

```bash
# Set your identity token
export IMI_IDENTITY_TOKEN="imi_tok_abc123..."

# Claim workspace for a project
iMi workspace claim my-project
# -> Creates /home/you/33GOD/workspaces/delorenj/my-project

# List YOUR workspaces (scoped to token)
iMi workspace list

# List ALL workspaces (global view)
iMi workspace list -g

# Audit who accessed your workspace
iMi workspace audit my-project
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

**Database connection issues**
```bash
# Verify PostgreSQL connection
psql $DATABASE_URL -c '\dt'
# Or check iMi database health
iMi doctor
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
