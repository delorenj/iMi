# 33GOD Component-Skill Dependency Map

**Version**: 1.0.0
**Purpose**: Track which Claude skills reference which 33GOD components for systematic updates
**Created**: 2026-01-21

## Overview

This document maps the dependencies between 33GOD ecosystem components and Claude skills. When a component undergoes architectural changes (e.g., iMi's PostgreSQL migration), this map ensures all relevant skills are updated systematically.

**Skill Directory**: `/home/delorenj/.config/zshyzsh/claude/skills/`

## Dependency Matrix

### iMi (Project Registry & Worktree Management)

| Skill | Reference Type | Update Priority | Notes |
|-------|---------------|-----------------|-------|
| `33god-imi-worktree-management` | **Primary** | CRITICAL | Extensively documents iMi architecture, commands, MCP tools, conventions |
| `33god-system-expert` | **Minor** | MEDIUM | Lists iMi as a component with brief description |
| `33god-development-lifecycle` | **Mention** | LOW | References iMi cluster detection (`.iMi/` directories) |

**Last Update**: 2026-01-21 (PostgreSQL migration, Project Registry role)

### Bloodbank (Event Bus)

| Skill | Reference Type | Update Priority | Notes |
|-------|---------------|-----------------|-------|
| `bloodbank-n8n-event-driven-workflows` | **Primary** | CRITICAL | Extensively documents event system, RabbitMQ, jelmore integration |
| `33god-workflow-generator` | **Integration** | MEDIUM | Documents Bloodbank event patterns, envelope structure |
| `33god-imi-worktree-management` | **Integration** | LOW | Mentions Bloodbank event publishing |
| `33god-system-expert` | **Minor** | LOW | Lists Bloodbank as a component |

**Last Update**: N/A (no recent changes)

### Jelmore (Execution Coordinator)

| Skill | Reference Type | Update Priority | Notes |
|-------|---------------|-----------------|-------|
| `bloodbank-n8n-event-driven-workflows` | **Primary** | CRITICAL | Extensively documents jelmore CLI, execution patterns, config files |
| `33god-imi-worktree-management` | **Integration** | LOW | Mentions Jelmore integration for session-aware worktree context |

**Last Update**: N/A (no recent changes)

### Flume (Task Orchestration)

| Skill | Reference Type | Update Priority | Notes |
|-------|---------------|-----------------|-------|
| `33god-imi-worktree-management` | **Integration** | LOW | Mentions Flume integration for task-worktree lifecycle |
| `33god-system-expert` | **Minor** | LOW | Lists Flume as a component with corporate hierarchy context |

**Last Update**: N/A (no recent changes)

### Yi (Agent Orchestrator)

| Skill | Reference Type | Update Priority | Notes |
|-------|---------------|-----------------|-------|
| `33god-system-expert` | **Minor** | LOW | Lists Yi as agent orchestrator, distinguishes from Jelmore/Flume |

**Last Update**: N/A (no recent changes)

### Node-RED (Workflow Automation)

| Skill | Reference Type | Update Priority | Notes |
|-------|---------------|-----------------|-------|
| `33god-workflow-generator` | **Primary** | CRITICAL | Extensively documents Node-RED flow patterns, node types, best practices |
| `33god-system-expert` | **Minor** | LOW | Lists Node-RED as workflow automation component |

**Last Update**: N/A (no recent changes)

### Other Components

| Component | Referenced By | Reference Type |
|-----------|--------------|----------------|
| Holocene (Dashboard) | `33god-system-expert` | Minor |
| Vernon (Voice App) | `33god-system-expert` | Minor |
| Agent Forge | `33god-system-expert` | Minor |

## Skills Without Component Dependencies

These skills focus on methodology, patterns, or external systems and don't directly reference 33GOD components:

- `33god-service-development` - Service creation guide (references registry.yaml)
- `bmad-*` skills - BMAD methodology skills (separate from 33GOD architecture)

## Update Priority Levels

**CRITICAL**: Skill extensively documents the component's architecture, APIs, or usage patterns. Must be updated when component undergoes major changes.

**MEDIUM**: Skill integrates with the component or documents integration patterns. Should be reviewed and updated when component APIs change.

**LOW**: Skill mentions the component in passing or lists it as part of the ecosystem. Update only if component's role fundamentally changes.

## Change Management Process

When a 33GOD component undergoes architectural changes:

1. **Identify affected skills** using this dependency map
2. **Prioritize updates** based on reference type (Primary → Integration → Minor → Mention)
3. **Update CRITICAL skills first** to ensure pipeline awareness
4. **Update MEDIUM skills** for integration correctness
5. **Update LOW skills** for completeness
6. **Update this map** with change date and notes

## Recent Changes Log

### 2026-01-21: iMi PostgreSQL Migration

**Component Change**: iMi migrated from SQLite to PostgreSQL, formalized as Project Registry

**Skills Updated**:
- ✅ `33god-imi-worktree-management` - Updated architecture sections, added Project Registry role, documented PostgreSQL features
- ✅ `33god-system-expert` - Updated component description to reflect Project Registry role

**Skills Not Updated** (no database implementation details):
- `33god-development-lifecycle` - References iMi conventions, not database

## Adding New Components

When adding a new 33GOD component:

1. Create or update relevant skills
2. Add component to this dependency map
3. Document reference types and update priorities
4. Link to canonical component documentation

## Adding New Skills

When creating a new skill that references 33GOD components:

1. Add skill to relevant component sections in this map
2. Specify reference type (Primary/Integration/Minor/Mention)
3. Document what aspect of the component is referenced
4. Set appropriate update priority

## Maintenance

This dependency map should be reviewed:
- **Immediately** after component architectural changes
- **Monthly** for completeness and accuracy
- **Before major releases** to ensure skill consistency

---

**Maintained by**: 33GOD Development Team
**Contact**: See `/home/delorenj/code/DeLoDocs/Projects/33GOD/` for system documentation
