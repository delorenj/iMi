---
tags:
  - project
  - AI
  - task
category: Project
description: iMi is a Rust-based Git worktree management tool designed for asynchronous, parallel multi-agent workflows, featuring opinionated defaults and real-time visibility into worktree activities.
title: iMi Git Worktree Management Tool
updated: 2025-06-28 17:10:37
created: 2025-06-28
---
# iMi

iMi is a worktree management tool made in Rust and is a component in [33GOD](33GOD.md), an agentic pipeline that combines a worktree management, a backend server built on FastAPI+Postgres and built from the ground up to facilitate asynchronous, parallel multi-agent tasks and development workflows.

## Features

- A component of the 33GOD Agentic Software Pipeline.
- Used by developers to lower friction in their daily workflows, while ensuring an organized and consistent project structure.
- Used by agents to enable conflict free paralellization of tasks, while minimizing the cognitive load of the orchestration.
- The need for this tool is increasing as the number and breadth of tasks being automated by agents increases.
- Git worktree management
- A real-time view of what's going on in your Git worktrees
  - Always know what repos are being worked on
  - Which branches in those repos
  - Which features are being worked on
  - Which agents are working on those features.
- Async workflow management
- Opinionated defaults - no config required

## Conventions

Worktrees are named with [prefix]-[branch]

### Prefixes

- trunk: Required for the main branch worktree (e.g. trunk-main, trunk-master, etc)
- feat: for new feature development
- pr: for checking out existing PRs either to review or patch
- review: Sematic dupe of pr for reviewing a teammate's PR or branch
- fix: for bug fixes
- aiops: Tasks specific to creating or modifying agents, rules, mcp configs, workflows, etc.
- devops: Tasks specific to dev QoL improvements, CI, repo organization, deploys, etc.

## Default Workflow

```bash
iMi feature add-login
```

Result:
 - if this feature branch is already checked out it'll set the context and CD into that directory
- if it's not already checked out it'll create the work tree and cd in to that directory
- there can only be one work tree sandbox checked out per repository per host machine
- the iMi application keep track of all repository host checkouts in an internal database
- multiple IMI apps can be networked together so the pipeline can distribute agentic load across hosts.
Convention over configuration simplifies your workflow.
- the paths are always implicit due to Convention you only need to reference the project
- everything is relative to the 33God root path, and there is only one clone per repo although the cloned may contain many work tree branches in addition to the main trunk branch
- there is always at least the main trunk present in the repo worktree sandbox
- features always branched from a freshly fetched `trunk`, in this case `trunk-main`
- pr suggestions are labeled `pr-suggestion-123-widget-1` and are always based on the `pr` being reviewed.
- fixes are labeled `fix`


## Examples of Manual Equivalents

### Reviewing a Pull Request

**manual:**

```
gh pr checkout 312 --worktree ../review-pr312
cd ../review-pr312
cp ../dotfiles/.env .
```

**iMi:**(within a gh repo)

```
iMi review 312
```

**iMi:** (from anywhere)
```
iMi repoName review 312
```

 if already checked out it will use that directory otherwise it will check out into the default pipeline sandbox directory
 
 ### Implementing a New Feature

**manual:**

```
git worktree add ../feat-some-new-feature -b feat/some-new-feature --checkout main
cd ../feat-some-new-feature
cp ../dotfiles/.env .
```

**iMi:**

```
iMi feat some-new-feature
```

### Implementing a Bugfix

**manual:**
```
git worktree add ../fix-some-bugfix -b fix/some-bugfix --checkout main
cd ../fix-some-bugfix
cp ../dotfiles/.env .
```

iMi:
```
iMi fix some-bugfix
```

## The Standard Directory Structure

This is the cornerstone of the whole strategy. We abandon the global `~/worktrees` folder in favor of a per-repo structure. All worktrees will be created as sibling directories to your main clone.

Let's say you have `~/code/my-awesome-project`. Your directory structure will look like this:

```plaintext
~/code/
└── my-awesome-project/          
    |- trunk-main                <-- This is your original `git clone` directory.
    ├── .git/                    <-- The REAL .git directory with all the data.
    ├── src/
    └── package.json
    └── .env                     <-- dotfiles are symlinked from a common worktree path
    └── .jarad-config            <-- personal config files are symlinked from a common worktree path

└── feat-new-api/                <-- A worktree for a new feature branch 'feat/new-api'.
    ├── .git                     <-- Just a FILE pointing to the real .git dir above.
    ├── src/
    └── package.json
    └── .env                     <-- dotfiles are symlinked from a common worktree path
    └── .jarad-config            <-- personal config files are symlinked from a common worktree path

└── pr-451/                      <-- A worktree for a PR review on branch feat/some-pr-branch.
    ├── .git                     <-- Also a file pointing back.
    ├── src/
    └── package.json
    └── .env                     <-- dotfiles are symlinked from a common worktree path
    └── .jarad-config            <-- personal config files are symlinked from a common worktree path

└── sync/                        <-- Each iMi repo has a "sync" folder with strict conflict resolution rules.
    ├── global                   <-- You can sync globally across entire 33GOD pipeline
    ├────── coding-rules.md      <-- Syncing is implemented using syncthing
    ├────── stack-specific.md
    ├── repo                     <-- Or you can sync per-repo
    ├────── .env                 
    ├────── .jarad-config
    ├────── memory-stuff


