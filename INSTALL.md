# iMi - Installation & Usage Guide

## Prerequisites

Before installing iMi, ensure you have:

1. **Rust Toolchain** (required)
   ```bash
   # Install Rust if you don't have it
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   
   # Restart your shell or run:
   source $HOME/.cargo/env
   
   # Verify installation
   rustc --version
   cargo --version
   ```

2. **Git** (required)
   ```bash
   # Verify Git is installed
   git --version
   
   # If not installed:
   # Ubuntu/Debian:
   sudo apt install git
   
   # macOS:
   brew install git
   ```

   ```bash
   ```

## Installation Methods

### Method 1: Install from Source (Recommended for Development)

This is the method to use since you have the source code locally:

```bash
# Navigate to the iMi directory
cd /home/delorenj/code/projects/33GOD/iMi/fix-feat

# Build and install
cargo install --path .

# This will:
# - Compile the project in release mode
# - Install the binary to ~/.cargo/bin/iMi
# - Make it available system-wide
```

### Method 2: Build Without Installing

If you want to test without installing:

```bash
cd /home/delorenj/code/projects/33GOD/iMi/fix-feat

# Build in release mode
cargo build --release

# The binary will be at:
# ./target/release/iMi

# Run it directly:
./target/release/iMi --help

# Or create a symlink:
sudo ln -s $(pwd)/target/release/iMi /usr/local/bin/iMi
```

### Method 3: Development Mode

For development and testing:

```bash
cd /home/delorenj/code/projects/33GOD/iMi/fix-feat

# Run without building
cargo run -- --help

# Example: Create a feature worktree
cargo run -- feat my-feature

# Run tests
cargo test
```

## Verification

After installation, verify iMi is working:

```bash
# Check if iMi is in your PATH
which iMi

# Should show: /home/delorenj/.cargo/bin/iMi

# Verify it runs
iMi --help

# Check version
iMi --version
```

## Quick Start

### 1. Initialize in a Git Repository

Navigate to a Git repository and create a trunk worktree:

```bash
# Go to your project
cd ~/code/my-project

# Make sure it's a git repository
git status

# If not a git repo, initialize one:
git init
git remote add origin <your-repo-url>

# Create a trunk worktree (must be in a directory starting with 'trunk-')
# First, let's set up the proper structure:
cd ..
git clone <your-repo-url> trunk-main
cd trunk-main

# Initialize iMi
iMi init
```

### 2. Create Your First Worktree

```bash
# Create a feature worktree
iMi feat user-authentication

# This will create: ../feat-user-authentication/
# You can now cd into it and start working:
cd ../feat-user-authentication
```

### 3. Monitor Your Worktrees

```bash
# In any worktree, start monitoring
iMi monitor

# Or check status
iMi status

# List all worktrees
iMi list
```

## Usage Examples

### Common Workflows

#### Feature Development
```bash
# Create feature worktree
iMi feat payment-integration

# Work in it
cd ../feat-payment-integration
git checkout -b feature/payment-integration

# Do your work...
# Commit changes
git add .
git commit -m "feat: add payment integration"

# When done, remove the worktree
iMi remove feat-payment-integration
```

#### Code Review
```bash
# Review PR #123
iMi review 123

# This creates pr-123/ worktree
cd ../pr-123

# Review the code...
# Add review comments, test, etc.

# When done
cd ../trunk-main
iMi remove pr-123
```

#### Bug Fixes
```bash
# Create fix worktree
iMi fix auth-bug

cd ../fix-auth-bug
git checkout -b fix/auth-bug

# Fix the bug...
git add .
git commit -m "fix: resolve authentication issue"

# Push and create PR
git push origin fix/auth-bug
```

#### AI/DevOps Operations
```bash
# AI operations worktree
iMi aiops new-agent-config

# DevOps worktree
iMi devops update-ci-pipeline
```

### Monitoring Multiple Worktrees

```bash
# Terminal 1: Start monitoring
iMi monitor

# Terminal 2-N: Work in different worktrees
cd ../feat-payment-integration
# Make changes...

cd ../fix-auth-bug
# Make changes...

# The monitor will show all activities in real-time
```

## Configuration

### Default Configuration

On first run, iMi creates `~/.config/iMi/config.toml`:

```bash
# View configuration
cat ~/.config/iMi/config.toml

# Edit configuration
nano ~/.config/iMi/config.toml
```

### Custom Configuration

Example `~/.config/iMi/config.toml`:

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

symlink_files = [
    ".env",
    ".vscode/settings.json",
    ".gitignore.local",
    ".editorconfig"
]
```

## Directory Structure

After using iMi, your project structure will look like:

```
~/code/
└── my-project/
    ├── trunk-main/              # Main development (trunk)
    ├── feat-user-auth/          # Feature worktrees
    ├── feat-payment/
    ├── pr-123/                  # PR review worktrees
    ├── fix-auth-bug/            # Bug fix worktrees
    ├── aiops-agent-config/      # AI operations
    ├── devops-ci-update/        # DevOps operations
    └── sync/                    # Shared configuration
        ├── global/              # Global sync files
        │   ├── coding-rules.md
        │   └── stack-specific.md
        └── repo/                # Repository-specific
            ├── .env
            └── .vscode/
```

## Troubleshooting

### iMi Command Not Found

```bash
# Check if ~/.cargo/bin is in PATH
echo $PATH | grep .cargo/bin

# If not, add to ~/.bashrc or ~/.zshrc:
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

### Database Issues

```bash
# Check database location
ls -la ~/.config/iMi/iMi.db

# If corrupted, reset:
rm ~/.config/iMi/iMi.db
# iMi will recreate it on next run
```

### Permission Issues

```bash
# Ensure proper permissions
chmod 755 ~/.cargo/bin/iMi
chmod -R 755 ~/.config/iMi/
```

### Git Repository Not Found

```bash
# Ensure you're in a git repository
git status

# iMi must be run from within a trunk-* directory or
# in a repository that has been initialized with iMi
```

## Updating iMi

```bash
# Navigate to source directory
cd /home/delorenj/code/projects/33GOD/iMi/fix-feat

# Pull latest changes (if using git)
git pull

# Rebuild and reinstall
cargo install --path . --force
```

## Uninstalling

```bash
# Remove the binary
cargo uninstall iMi

# Remove configuration and database
rm -rf ~/.config/iMi/

# Remove any symlinks if you created them
sudo rm /usr/local/bin/iMi  # if you created this
```

## Advanced Usage

### Using with Multiple Repositories

```bash
# Specify repository explicitly
iMi --repo my-project feat new-feature

# Or set IMI_REPO environment variable
export IMI_REPO=my-project
iMi feat new-feature
```

### Integrating with Your Workflow

Add to your shell aliases (`~/.bashrc` or `~/.zshrc`):

```bash
# Quick aliases
alias if='iMi feat'
alias ifix='iMi fix'
alias ir='iMi review'
alias im='iMi monitor'
alias is='iMi status'
alias il='iMi list'

# Quick navigation
alias cdtrunk='cd $(find ~/code -maxdepth 2 -name "trunk-*" -type d | head -1)'
```

### CI/CD Integration

Use iMi in CI/CD pipelines:

```yaml
# .github/workflows/test.yml
jobs:
  test:
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
      - name: Install iMi
        run: cargo install --path .
      - name: Run tests
        run: iMi test
```

## Getting Help

```bash
# General help
iMi --help

# Command-specific help
iMi feat --help
iMi monitor --help
iMi init --help

# View all commands
iMi --help
```

## Next Steps

1. **Initialize your repository**: `iMi init` in your trunk worktree
2. **Create a feature worktree**: `iMi feat my-first-feature`
3. **Start monitoring**: `iMi monitor`
4. **Check the documentation**: See [README.md](README.md) for detailed features
5. **Explore commands**: Try `iMi --help` to see all available commands

## Support

- **Issues**: Report at GitHub Issues
- **Documentation**: Check the [GEMINI.md](GEMINI.md) for technical details
- **Tests**: Run `cargo test` to verify everything works

---

Happy coding with iMi! 🚀
