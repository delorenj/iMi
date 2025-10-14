---
modified: 2025-10-14T05:31:51-04:00
swarm-analysis: 2025-10-14T09:54:00-04:00
---

# iMi Test Session Analysis & Implementation Plan

## Critical Issues Identified

### Issue 1: Manual Directory Restructuring (Footnote 1)
**Problem**: When a user runs iMi in a non-trunk directory, they're given manual instructions to restructure, which is error-prone.

**Current Behavior**:
```sh
⚠  Current directory: eventflow
iMi works best when initialized from a 'trunk-*' directory.
Recommendation:
  cd ..
  mv eventflow trunk-main
  cd trunk-main
  iMi init
```

**Desired Behavior**: Automate this restructuring with a confirmation prompt.

### Issue 2: Dangerous Naming Collisions (Footnote 2)
**Problem**: The recommended manual process creates naming collisions:
- Multiple `trunk-main` directories could exist in `~/code`
- No clear indication which repo a `trunk-main` belongs to
- No sandbox containing all branches for a given project

**Solution Needed**: Create a parent directory named after the repo (e.g., `~/code/eventflow/trunk-main`)

### Issue 3: Poor Init UX (Footnote 3)
**Problems**:
1. Every subsequent init after the first shows a warning about config existing
2. No intelligent repo detection or TUI selector
3. Missing `iMi init <github-repo>` clone-and-setup functionality

**Desired Behavior**:
- Inside non-iMi repo: Auto-restructure and register
- Outside any repo with no args: Show TUI selector of existing repos
- Outside any repo with arg: Clone from GitHub and setup (e.g., `iMi init delorenj/eventflow`)

### Issue 4: Spurious Worktree Cleanup (Footnote 4)
**Problem**: Freshly created worktrees show cleanup messages:
```
🧹 Cleaning up worktree artifacts for: feat-addSound
🎯 Cleanup complete for: feat-addSound
🗑 Removing auto-created branch: feat-addSound
✅ Auto-created branch removed
```

**Root Cause**: `cleanup_worktree_artifacts` is being called unconditionally in `git.rs:121` even for brand new worktrees.

**Solution**: Only cleanup if there's an actual conflict or existing worktree.

### Issue 5: Incorrect Worktree Path & No Directory Change (Footnote 5)
**Problems**:
1. **Wrong path displayed**: Shows `/home/delorenj/code/feat-addSound` but should be `/home/delorenj/code/eventflow/feat-addSound`
2. **No directory change**: Says "Changed to directory" but pwd shows we're still in trunk-main

**Root Causes**:
1. Path construction in `worktree.rs:139-156` doesn't properly detect the IMI_PATH
2. No actual `cd` command is being executed after worktree creation

---

## Original Test Session

```sh
~/code
❯ cd eventflow

~/code/eventflow  mainvia 🥟 v1.2.22  via  v24.6.0
❯ ../projects/33GOD/iMi/fix-feat/QUICKSTART.sh
🚀 iMi Quick Start Guide
========================

✅ iMi is already installed at: /home/delorenj/.cargo/bin/iMi

📍 Current location: /home/delorenj/code/eventflow

✅ Git repository detected

⚠  Current directory: eventflow

iMi works best when initialized from a 'trunk-*' directory.

Recommendation:
  cd ..
  mv eventflow trunk-main
  cd trunk-main
  iMi init

Or continue anyway with: iMi init

📚 Quick Command Reference:
  iMi feat <name>      - Create feature worktree
  iMi fix <name>       - Create bugfix worktree
  iMi review <pr>      - Create PR review worktree
  iMi status           - Show all worktrees
  iMi monitor          - Monitor activities
  iMi --help           - Full help

📖 Full documentation:
  README.md   - Features and examples
  INSTALL.md  - Installation guide
  GEMINI.md   - Technical details

🎉 You're ready to use iMi!

~/code/eventflow  mainvia 🥟 v1.2.22  via  v24.6.0
❯ cd ..

~/code
❯ mv eventflow /tmp/trunk-main

~/code
❯ mkdir eventflow

~/code
❯ mv /tmp/trunk-main eventflow

~/code
❯ cd eventflow

~/code/eventflow
❯ ls
trunk-main

~/code/eventflow
❯ ls
trunk-main

~/code/eventflow
❯ cd trunk-main
mise WARN  Config files in ~/code/eventflow/trunk-main/mise.toml are not trusted.
Trust them with `mise trust`. See https://mise.jdx.dev/cli/trust.html for more information.
mise WARN  Config files in ~/code/eventflow/trunk-main/mise.toml are not trusted.
Trust them with `mise trust`. See https://mise.jdx.dev/cli/trust.html for more information.

code/eventflow/trunk-main  mainvia 🥟 v1.2.22  via  v24.6.0
❯ mt
mise trusted /home/delorenj/code/eventflow/trunk-main

code/eventflow/trunk-main  mainvia 🥟 v1.2.22  via  v24.6.0
❯ ../../projects/33GOD/iMi/fix-feat/QUICKSTART.sh
🚀 iMi Quick Start Guide
========================

✅ iMi is already installed at: /home/delorenj/.cargo/bin/iMi

📍 Current location: /home/delorenj/code/eventflow/trunk-main

✅ Git repository detected

✅ Correct directory naming: trunk-main

Would you like to initialize iMi here? (y/n)
y

Initializing iMi...
🌍 Running outside of a git repository. Setting up global iMi configuration...
ℹ Configuration already exists at /home/delorenj/.config/iMi/config.toml. Use --force to overwrite
ℹ Database already exists at /tmp/.tmpzopMU2/imi.db. Use --force to overwrite.
🚀 Running inside a git repository. Initializing...
✅ Registered repository 'eventflow' in the database.
✅ Created .iMi directory at /home/delorenj/code/eventflow/.iMi
Successfully initialized iMi for repository 'eventflow'.

✅ iMi initialized!

📚 Quick Command Reference:
  iMi feat <name>      - Create feature worktree
  iMi fix <name>       - Create bugfix worktree
  iMi review <pr>      - Create PR review worktree
  iMi status           - Show all worktrees
  iMi monitor          - Monitor activities
  iMi --help           - Full help

📖 Full documentation:
  README.md   - Features and examples
  INSTALL.md  - Installation guide
  GEMINI.md   - Technical details

🎉 You're ready to use iMi!

code/eventflow/trunk-main  mainvia 🥟 v1.2.22  via  v24.6.0 took 19s
❯ vi ~/.config/iMi/config.toml

code/eventflow/trunk-main  mainvia 🥟 v1.2.22  via  v24.6.0 took 21s
❯ ls
bun.lockb         generate-favicon.js  package.json       README.md           tsconfig.json
components.json   index.html           package-lock.json  src                 tsconfig.node.json
docs              mise.toml            postcss.config.js  tailwind.config.ts  vite.config.ts
eslint.config.js  node_modules         public             tsconfig.app.json

code/eventflow/trunk-main  mainvia 🥟 v1.2.22  via  v24.6.0
❯ imi status
📊 Worktree Status
ℹ No active worktrees found

code/eventflow/trunk-main  mainvia 🥟 v1.2.22  via  v24.6.0
❯ imi feat addSound
🚀 Creating feature worktree: addSound
🧹 Cleaning up worktree artifacts for: feat-addSound
🎯 Cleanup complete for: feat-addSound
🗑 Removing auto-created branch: feat-addSound
✅ Auto-created branch removed
✅ Worktree created successfully
✅ Feature worktree created at: /home/delorenj/code/feat-addSound
📁 Changed to directory: /home/delorenj/code/feat-addSound

code/eventflow/trunk-main  mainvia 🥟 v1.2.22  via  v24.6.0
❯ ls
bun.lockb         generate-favicon.js  package.json       README.md           tsconfig.json
components.json   index.html           package-lock.json  src                 tsconfig.node.json
docs              mise.toml            postcss.config.js  tailwind.config.ts  vite.config.ts
eslint.config.js  node_modules         public             tsconfig.app.json

code/eventflow/trunk-main  mainvia 🥟 v1.2.22  via  v24.6.0
❯ cd ..

~/code/eventflow
❯ ls
trunk-main

~/code/eventflow
❯ cd -
~/code/eventflow/trunk-main

code/eventflow/trunk-main  mainvia 🥟 v1.2.22  via  v24.6.0
❯ ls
bun.lockb         generate-favicon.js  package.json       README.md           tsconfig.json
components.json   index.html           package-lock.json  src                 tsconfig.node.json
docs              mise.toml            postcss.config.js  tailwind.config.ts  vite.config.ts
eslint.config.js  node_modules         public             tsconfig.app.json

code/eventflow/trunk-main  mainvia 🥟 v1.2.22  via  v24.6.0
```
