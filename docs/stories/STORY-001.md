# STORY-001: Add Project Create Command to iMi CLI

**Epic:** Project Management Automation
**Priority:** Must Have
**Story Points:** 8
**Status:** Not Started
**Assigned To:** Unassigned
**Created:** 2025-12-27
**Sprint:** Sprint 1

---

## User Story

As a developer using the iMi ecosystem
I want to create new projects with a single CLI command
So that I can instantly bootstrap properly-configured repositories with ecosystem-standard boilerplate

---

## Description

### Background
Currently, developers must manually create GitHub repositories, set up boilerplate files, configure mise, initialize docker compose, and establish project structure for each new project. This repetitive work wastes time and introduces inconsistency across the iMi ecosystem.

This story adds a `project create` command to the existing iMi CLI (Rust crate) that automates the entire project bootstrapping process based on natural language descriptions, structured JSON input, or PRD documents.

### Scope

**In scope:**
- CLI command: `imi project create` with multiple input modes:
  - `--concept|-c` flag with natural language description
  - `--name|-n` flag for explicit project naming
  - `--prd|-p` flag accepting markdown file path
- Bloodbank event handler: `imi.project.create` accepting JSON payloads
- Intelligent stack detection and template selection:
  - Python projects with FastAPI/UV/hatchling
  - React projects with bun/TypeScript/Tailwind/Vite/shadcn
  - Generic projects with best-effort AI interpretation
- GitHub repository creation (public by default, unless specified)
- Boilerplate scaffolding:
  - `mise.toml` configuration
  - `.mise/tasks` directory with common tasks
  - Python: `pyproject.toml` with UV/hatchling setup
  - React: `package.json` with bun/TypeScript/Vite
  - Docker compose with `compose.yml`
  - Sensible README generation
- Database service configuration:
  - PostgreSQL: `192.168.1.12:5432` with `$DEFAULT_USERNAME:$DEFAULT_PASSWORD`
  - Redis: `192.168.1.12:6743` (passwordless)
  - Qdrant: `qdrant.delo.sh`
- Project naming logic (explicit name, extracted from PRD/concept, or AI-generated)

**Out of scope:**
- Private repository creation (future enhancement)
- Custom template creation UI (stick to hardcoded templates for now)
- Multi-repository orchestration (monorepo support)
- CI/CD pipeline setup beyond basic structure
- Custom database credentials (use ecosystem defaults)

### User Flow

**CLI Flow (--concept flag):**
1. Developer runs: `imi project create --concept "An android app that helps you plan lego builds. Use Flutter." --name "LegoJoe"`
2. iMi detects stack requirements (Flutter, mobile)
3. iMi generates project name if not provided (or uses "LegoJoe")
4. iMi creates GitHub repository `LegoJoe`
5. iMi scaffolds Flutter project structure locally
6. iMi adds `mise.toml`, `.mise/tasks`, `compose.yml`
7. iMi generates README based on concept
8. iMi initializes git, commits initial structure
9. iMi pushes to GitHub
10. Developer sees success message with repo URL

**CLI Flow (--prd flag):**
1. Developer runs: `imi project create --prd /path/to/prd.md`
2. iMi reads PRD markdown file
3. iMi extracts project name from PRD or generates one
4. iMi detects stack requirements from PRD content
5. iMi follows steps 4-10 from concept flow

**Bloodbank Event Flow:**
1. External system publishes `imi.project.create` event to RabbitMQ
2. iMi Bloodbank handler receives event with JSON payload
3. iMi parses JSON structure for project requirements
4. iMi follows standard project creation flow
5. iMi publishes completion event to Bloodbank

---

## Acceptance Criteria

- [ ] CLI accepts `imi project create --concept "description" --name "ProjectName"` syntax
- [ ] CLI accepts `imi project create --prd /path/to/file.md` syntax
- [ ] Bloodbank handler accepts `imi.project.create` events with JSON payloads
- [ ] Project name extraction works from:
  - Explicit `--name` flag (highest priority)
  - PRD document content
  - Concept description
  - AI-generated fallback
- [ ] Stack detection correctly identifies:
  - Python/FastAPI projects → UV/hatchling setup
  - React projects → bun/TypeScript/Vite/Tailwind/shadcn setup
  - Generic projects → reasonable defaults
- [ ] GitHub repository created successfully:
  - Public visibility by default
  - Correct repository name
  - Initial commit with all boilerplate
- [ ] Python projects include:
  - `pyproject.toml` with UV/hatchling configuration
  - `mise.toml` with Python version and tasks
  - `.mise/tasks/` with common Python tasks (test, lint, run)
  - `compose.yml` if database dependencies detected
- [ ] React projects include:
  - `package.json` with bun as package manager
  - TypeScript configuration (`tsconfig.json`)
  - Vite configuration (`vite.config.ts`)
  - Tailwind CSS setup (`tailwind.config.js`)
  - shadcn-ui initialization
  - `mise.toml` with Node version and tasks
  - `.mise/tasks/` with common frontend tasks (dev, build, test)
- [ ] Docker compose configuration:
  - PostgreSQL service points to `192.168.1.12:5432` (external_link pattern)
  - Redis service points to `192.168.1.12:6743` (external_link pattern)
  - Qdrant service points to `qdrant.delo.sh` if vector DB required
- [ ] README generated with:
  - Project name and description
  - Getting started instructions
  - mise task documentation
  - Stack information
- [ ] Error handling for:
  - GitHub API failures (auth, rate limits, existing repo name)
  - Invalid PRD file paths
  - Malformed JSON payloads from Bloodbank
  - Missing required dependencies (gh CLI, git)
- [ ] Success output includes:
  - Repository URL
  - Local project path
  - Next steps (mise install, mise run dev)

---

## Technical Notes

### Components

**New Rust modules to add:**
- `src/commands/project/create.rs` - Main command implementation
- `src/commands/project/mod.rs` - Project command module
- `src/templates/` - Template generation logic
  - `src/templates/python.rs`
  - `src/templates/react.rs`
  - `src/templates/generic.rs`
- `src/github.rs` - GitHub API integration
- `src/bloodbank/handlers/project_create.rs` - Bloodbank event handler

**Existing modules to modify:**
- `src/cli.rs` - Add `project` subcommand
- `src/main.rs` - Wire up new command

### External Dependencies

**Cargo.toml additions:**
```toml
[dependencies]
octocrab = "0.38"  # GitHub API client
serde_yaml = "0.9"  # YAML parsing for mise.toml generation
tera = "1.19"  # Template engine for file generation
tokio = { version = "1.35", features = ["full"] }
reqwest = { version = "0.11", features = ["json"] }
```

### GitHub API Integration

**Required GitHub operations:**
1. Create repository: `POST /user/repos`
   - Requires GitHub token from environment (`GITHUB_TOKEN`)
   - Set visibility to public
   - Initialize with README

2. Error handling:
   - 401: Invalid/missing token → clear error message
   - 422: Repository name already exists → suggest alternatives
   - 403: Rate limit exceeded → wait and retry with exponential backoff

**GitHub CLI dependency:**
- Leverage `gh` CLI if available for authentication
- Fallback to `GITHUB_TOKEN` environment variable
- Check `gh auth status` before attempting operations

### Template Structure

**Template data model:**
```rust
struct ProjectConfig {
    name: String,
    description: String,
    stack: StackType,
    databases: Vec<DatabaseType>,
    services: Vec<ServiceType>,
}

enum StackType {
    PythonFastAPI,
    ReactVite,
    Generic(String),
}

enum DatabaseType {
    Postgres { host: String, port: u16, user: String, password: String },
    Redis { host: String, port: u16 },
    Qdrant { url: String },
}
```

**Template files to generate:**
- `templates/mise.toml.tera`
- `templates/python/pyproject.toml.tera`
- `templates/react/package.json.tera`
- `templates/react/tsconfig.json.tera`
- `templates/compose.yml.tera`
- `templates/README.md.tera`

### Bloodbank Integration

**Event schema:**
```json
{
  "event_type": "imi.project.create",
  "payload": {
    "name": "optional-project-name",
    "concept": "optional-description",
    "prd_path": "optional-path",
    "stack": {
      "api": "FastAPI",
      "frontend": "react dashboard"
    },
    "mise_tasks": ["hello-world"]
  }
}
```

**Handler requirements:**
- Subscribe to `imi.project.create` queue
- Parse JSON payload with serde_json
- Map payload to `ProjectConfig`
- Execute project creation logic
- Publish success/failure event to `imi.project.create.result`

### Database Configuration

**Environment variables required:**
```bash
DEFAULT_USERNAME=postgres  # For PostgreSQL default user
DEFAULT_PASSWORD=<secure-password>  # For PostgreSQL default password
GITHUB_TOKEN=<gh-token>  # For GitHub API authentication
```

**Docker compose patterns:**
```yaml
# PostgreSQL (external service)
services:
  db:
    image: postgres:16
    external_links:
      - "192.168.1.12:5432:postgres"
    environment:
      POSTGRES_USER: ${DEFAULT_USERNAME}
      POSTGRES_PASSWORD: ${DEFAULT_PASSWORD}

# Redis (external service)
services:
  redis:
    image: redis:7
    external_links:
      - "192.168.1.12:6743:redis"
```

### AI/LLM Integration (Future)

**Current scope: Rule-based stack detection**
- Parse concept/PRD text for keywords (Python, FastAPI, React, Flutter, etc.)
- Use simple heuristics for template selection
- No LLM calls required initially

**Future enhancement:**
- Claude API integration for intelligent stack analysis
- Template customization based on nuanced requirements
- Conversation-based project refinement

### Error Handling Strategy

**Fatal errors (abort creation):**
- GitHub token missing/invalid
- Repository name conflict
- Git operations fail (commit, push)

**Warnings (proceed with defaults):**
- Unknown stack type → use generic template
- Missing optional fields → use sensible defaults
- Database services unavailable → note in README

### CLI Output Format

```
Creating project: LegoJoe
  ✓ Detected stack: Flutter (mobile)
  ✓ Generated project structure
  ✓ Created GitHub repository: https://github.com/username/LegoJoe
  ✓ Initialized git repository
  ✓ Added boilerplate files
  ✓ Committed initial structure
  ✓ Pushed to GitHub

Success! Your project is ready.

Repository: https://github.com/username/LegoJoe
Local path: /path/to/LegoJoe

Next steps:
  cd LegoJoe
  mise install
  mise run dev
```

---

## Dependencies

**Prerequisite Stories:**
- None (this is STORY-001)

**Prerequisite Infrastructure:**
- GitHub account with API access (existing)
- PostgreSQL running on `192.168.1.12:5432` (existing)
- Redis running on `192.168.1.12:6743` (existing)
- Qdrant service at `qdrant.delo.sh` (existing)
- Bloodbank (RabbitMQ) running (existing)

**Blocked Stories:**
- STORY-002: Add private repository support (depends on this story)
- STORY-003: Custom template management (depends on this story)
- STORY-004: Multi-repository orchestration (depends on this story)

**External Dependencies:**
- `gh` CLI tool (optional but recommended)
- Git installed locally
- Rust toolchain with cargo
- Internet connectivity for GitHub API

---

## Definition of Done

- [ ] Code implemented in new Rust modules:
  - [ ] `src/commands/project/create.rs`
  - [ ] `src/templates/*.rs`
  - [ ] `src/github.rs`
  - [ ] Bloodbank handler module
- [ ] CLI command wired up in `src/cli.rs` and `src/main.rs`
- [ ] Unit tests written and passing (≥80% coverage):
  - [ ] Stack detection logic tests
  - [ ] Template generation tests
  - [ ] GitHub API interaction tests (mocked)
  - [ ] Bloodbank event parsing tests
- [ ] Integration tests passing:
  - [ ] End-to-end project creation flow (uses test GitHub repo)
  - [ ] Error handling scenarios
- [ ] Template files created and tested:
  - [ ] Python project templates
  - [ ] React project templates
  - [ ] Generic project templates
  - [ ] Docker compose templates
- [ ] Documentation updated:
  - [ ] README.md with `imi project create` usage examples
  - [ ] TASK.md examples validated
  - [ ] Template documentation
- [ ] Code reviewed and approved (1+ reviewer)
- [ ] All acceptance criteria validated (✓)
- [ ] Manual testing completed:
  - [ ] Create Python/FastAPI project
  - [ ] Create React/TypeScript project
  - [ ] Create project from PRD file
  - [ ] Trigger via Bloodbank event
  - [ ] Verify GitHub repository created correctly
  - [ ] Verify boilerplate files present and valid
  - [ ] Verify mise tasks work
  - [ ] Test error cases (invalid token, existing repo, etc.)
- [ ] Merged to main branch
- [ ] Feature available in next release

---

## Story Points Breakdown

- **Rust command implementation:** 2 points
  - CLI argument parsing
  - Command structure and routing
- **Template system:** 3 points
  - Template engine integration (Tera)
  - Python/React/Generic templates
  - File generation logic
- **GitHub integration:** 2 points
  - octocrab setup
  - Repository creation
  - Error handling
- **Bloodbank handler:** 1 point
  - Event subscription
  - JSON parsing
  - Handler wiring
- **Testing:** 3 points
  - Unit tests (stack detection, templates)
  - Integration tests (end-to-end flow)
  - Manual testing scenarios
- **Documentation:** 1 point
  - README updates
  - Template docs
- **Total:** 12 points → **Rounded to 8 (reduced scope to MVP)**

**Rationale:** Initial estimate of 12 points is too large for a single story. To fit within sprint capacity (≤8 points), reduce scope to:
- Focus on Python and React templates (defer Flutter/other stacks)
- Implement basic Bloodbank handler (defer advanced event patterns)
- Generate minimal but functional README (defer rich formatting)
- Use simple keyword-based stack detection (defer AI integration)

With scope reduction, **8 points is achievable** within a single sprint for a senior engineer.

---

## Additional Notes

### Ecosystem Integration

This command integrates with the broader 33GOD/iMi ecosystem:
- **iMi (this project):** Worktree management + project creation
- **Bloodbank:** Event-driven orchestration for cross-system workflows
- **Flume:** Session/task tracking (could track project creation as task)
- **33GOD agents:** Future enhancement to use agents for intelligent project analysis

### Future Enhancements (Out of Scope for STORY-001)

**STORY-002: Private repository support**
- Add `--visibility` flag (public, private, internal)
- Handle GitHub pricing tier limitations

**STORY-003: Custom template management**
- Template registry in `~/.config/imi/templates/`
- `imi template add` command
- Template versioning

**STORY-004: Interactive project creation**
- Conversational CLI flow with prompts
- `imi project create --interactive`
- Guided stack selection

**STORY-005: Project update/sync**
- `imi project sync` to update boilerplate
- Template versioning and migration
- Incremental updates without overwriting custom code

### Risk Assessment

**High risk:**
- GitHub API rate limiting during testing (mitigation: use test account, implement backoff)
- Template complexity explosion (mitigation: start with 2-3 core templates, iterate)

**Medium risk:**
- Bloodbank integration complexity (mitigation: defer advanced patterns to future story)
- Stack detection accuracy (mitigation: allow manual override with flags)

**Low risk:**
- Mise configuration generation (well-understood format)
- Docker compose setup (existing patterns to follow)

---

## Progress Tracking

**Status History:**
- 2025-12-27: Created by Jarad DeLorenzo (BMAD Scrum Master workflow)

**Actual Effort:** TBD (will be filled during/after implementation)

---

**This story was created using BMAD Method v6 - Phase 4 (Implementation Planning)**
