# TASK

Pruning stale worktree references in `imi` should work as expected.
Although, just a normal git worktree prune doesn't seem to work either (see below).

AFIK, the feat-pr-validation-fix worktree was deleted manually outside of git, so both `imi prune` and `git worktree prune` should be able to clean up the stale reference.

```sh
~/code/iMi  delorenj in 🌐 big-chungus in
❯ imi -v
imi 0.2.0

~/code/iMi  delorenj in 🌐 big-chungus in
❯ imi prune
🧹 Cleaning up stale worktree references
Error: Git repository not found at path: /home/delorenj/code/iMi

~/code/iMi  delorenj in 🌐 big-chungus in
❯ cd trunk-main

code/iMi/trunk-main  main$ delorenj in 🌐 big-chungus in is 📦 v0.2.0  via 🦀 v1.89.0
❯ imi prune
🧹 Cleaning up stale worktree references
✅ Cleanup complete

code/iMi/trunk-main  main$ delorenj in 🌐 big-chungus in is 📦 v0.2.0  via 🦀 v1.89.0
❯ ls ..
feat-pr-validation-fix  pr-458  sync  trunk-main

code/iMi/trunk-main  main$ delorenj in 🌐 big-chungus in is 📦 v0.2.0  via 🦀 v1.89.0
❯ cd ../feat-pr-validation-fix
mise WARN  Config files in ~/code/iMi/feat-pr-validation-fix/.mise.toml are not trusted.
Trust them with `mise trust`. See https://mise.jdx.dev/cli/trust.html for more information.
mise WARN  Config files in ~/code/iMi/feat-pr-validation-fix/.mise.toml are not trusted.
Trust them with `mise trust`. See https://mise.jdx.dev/cli/trust.html for more information.

code/iMi/feat-pr-validation-fix  feat/pr-validation-fix$ delorenj in 🌐 big-chungus in is 📦 v0.1.0  via 🦀 v1.89.0
❯ mt
mise trusted /home/delorenj/code/iMi/feat-pr-validation-fix

code/iMi/feat-pr-validation-fix  feat/pr-validation-fix$ delorenj in 🌐 big-chungus in is 📦 v0.1.0  via 🦀 v1.89.0
❯ git status
On branch feat/pr-validation-fix
Your branch is up to date with 'origin/feat/pr-validation-fix'.

nothing to commit, working tree clean

code/iMi/feat-pr-validation-fix  feat/pr-validation-fix$ delorenj in 🌐 big-chungus in is 📦 v0.1.0  via 🦀 v1.89.0
❯ git push
Everything up-to-date

code/iMi/feat-pr-validation-fix  feat/pr-validation-fix$ delorenj in 🌐 big-chungus in is 📦 v0.1.0  via 🦀 v1.89.0
❯ cd -
~/code/iMi/trunk-main

code/iMi/trunk-main  main$ delorenj in 🌐 big-chungus in is 📦 v0.2.0  via 🦀 v1.89.0
❯ gwt list
/home/delorenj/code/iMi/trunk-main              0e95400 [main]
/home/delorenj/code/iMi/feat-pr-validation-fix  204c863 [feat/pr-validation-fix]

code/iMi/trunk-main  main$ delorenj in 🌐 big-chungus in is 📦 v0.2.0  via 🦀 v1.89.0
❯ gwt prune

code/iMi/trunk-main  main$ delorenj in 🌐 big-chungus in is 📦 v0.2.0  via 🦀 v1.89.0
❯ gwt list
/home/delorenj/code/iMi/trunk-main              0e95400 [main]
/home/delorenj/code/iMi/feat-pr-validation-fix  204c863 [feat/pr-validation-fix]

```
