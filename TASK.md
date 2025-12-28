# Command: project create

## Progress

### Phase 1: JSON Output Support ✅ COMPLETE
- Added global `--json` flag to CLI (src/cli.rs:22)
- Implemented JsonResponse helper structure (src/main.rs:29-58)
- Updated all 16 command handlers to support JSON output
- Tested and verified JSON mode works across all commands
- Non-JSON (colored terminal) mode preserved and working

### Next: Phase 2 - FastMCP Server
Ready to begin MCP server implementation per implementation plan.

## Usage

```
imi project create [--concept|-c] "An android app that helps you plan lego builds. Use Flutter." [--name|-n] "LegoJoe"

# Use a markdown doc to describe concept
iMi prooject create [--prd|-p] /some/markdown.md #if name is null, will look to prd or concept for explicit name and fallback to deciding on its own.

# Or use Bloodbank command
imi.project.create

{
  "concept": "Blah Blah",
  "name": "SomeProjectName"
}

# Can also use arbitrary structured json data to describe anything
# Vague stuff will just be guessed. Wrong? Who cares! It's awesome.

{
 "name": "MyProject",
 "api": "FastAPI",
 "frontend": "react dashboard",
 "mise-tasks": [
  "hello-world"
  ]
}
```

## Instructions

Use bmad workflow.
Spawn a staff level architect to answer questions for me.
Best guesses are fine.
Continue from phase to phase until acceptance criteria are met

## Acceptance Criteria

Above example commands result in:

- new gh repo for each `create`
- works via CLI or Bloodbank command
- gh repo contains the base boilerplate for whatever stack the repo requires
  - Bootstrapped with mise.toml, .mise/tasks
  - Python apps bootstrapped with UV, hatchling packaging
  - React apps bootstrapped with `bun`, Typescript, tailwindcss, vite, shadcn
  - Containerization with docker compose, `compose.yml`.
  - If postgres required, use native postgres on 192.168.1.12:5432 $DEFAULT_USERNAME:$DEFAULT_PASSWORD
  - If redis required, use native passwordless redis on 192.168.1.12:6743
  - If qdrant required, use qdrant.delo.sh
  - Sensible readme added
  - Public repo, unless otherwise specified
