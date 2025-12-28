use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;
use tera::{Context, Tera};

use super::config::{DatabaseType, ProjectConfig, StackType};

pub struct TemplateEngine {
    tera: Tera,
}

impl TemplateEngine {
    pub fn new() -> Result<Self> {
        let mut tera = Tera::default();

        // Register inline templates
        tera.add_raw_templates(vec![
            ("mise.toml", MISE_TEMPLATE),
            ("python/pyproject.toml", PYTHON_PYPROJECT_TEMPLATE),
            ("react/package.json", REACT_PACKAGE_JSON_TEMPLATE),
            ("react/tsconfig.json", REACT_TSCONFIG_TEMPLATE),
            ("react/vite.config.ts", REACT_VITE_CONFIG_TEMPLATE),
            ("react/tailwind.config.js", REACT_TAILWIND_CONFIG_TEMPLATE),
            ("compose.yml", DOCKER_COMPOSE_TEMPLATE),
            ("README.md", README_TEMPLATE),
        ])?;

        Ok(Self { tera })
    }

    pub fn render_mise_toml(&self, config: &ProjectConfig) -> Result<String> {
        let mut context = Context::new();
        context.insert("project_name", &config.name);
        context.insert("stack", &config.stack);

        // Determine tooling versions
        let (tools, env_vars) = match &config.stack {
            StackType::PythonFastAPI => (
                vec![("python", "3.12"), ("uv", "latest")],
                vec![("VIRTUAL_ENV", ".venv")],
            ),
            StackType::ReactVite => (
                vec![("node", "20"), ("bun", "latest")],
                vec![],
            ),
            StackType::Generic { .. } => (vec![], vec![]),
        };

        context.insert("tools", &tools);
        context.insert("env_vars", &env_vars);
        context.insert("tasks", &config.mise_tasks);

        Ok(self.tera.render("mise.toml", &context)?)
    }

    pub fn render_python_pyproject(&self, config: &ProjectConfig) -> Result<String> {
        let mut context = Context::new();
        context.insert("project_name", &config.name);
        context.insert("description", &config.description);

        let dependencies = vec![
            "fastapi[standard]",
            "uvicorn[standard]",
            "pydantic",
            "pydantic-settings",
        ];
        context.insert("dependencies", &dependencies);

        Ok(self.tera.render("python/pyproject.toml", &context)?)
    }

    pub fn render_react_package_json(&self, config: &ProjectConfig) -> Result<String> {
        let mut context = Context::new();
        context.insert("project_name", &config.name);
        context.insert("description", &config.description);

        Ok(self.tera.render("react/package.json", &context)?)
    }

    pub fn render_react_tsconfig(&self, _config: &ProjectConfig) -> Result<String> {
        Ok(self.tera.render("react/tsconfig.json", &Context::new())?)
    }

    pub fn render_react_vite_config(&self, _config: &ProjectConfig) -> Result<String> {
        Ok(self.tera.render("react/vite.config.ts", &Context::new())?)
    }

    pub fn render_react_tailwind_config(&self, _config: &ProjectConfig) -> Result<String> {
        Ok(self
            .tera
            .render("react/tailwind.config.js", &Context::new())?)
    }

    pub fn render_docker_compose(&self, config: &ProjectConfig) -> Result<String> {
        if config.databases.is_empty() {
            return Ok(String::new());
        }

        let mut context = Context::new();
        context.insert("project_name", &config.name);
        context.insert("databases", &config.databases);

        Ok(self.tera.render("compose.yml", &context)?)
    }

    pub fn render_readme(&self, config: &ProjectConfig) -> Result<String> {
        let mut context = Context::new();
        context.insert("project_name", &config.name);
        context.insert("description", &config.description);
        context.insert("stack", &config.stack);
        context.insert("mise_tasks", &config.mise_tasks);
        context.insert("has_databases", &(!config.databases.is_empty()));

        Ok(self.tera.render("README.md", &context)?)
    }
}

// Template constants

const MISE_TEMPLATE: &str = r#"[tools]
{% for tool in tools -%}
{{ tool.0 }} = "{{ tool.1 }}"
{% endfor %}
[env]
{% for env in env_vars -%}
{{ env.0 }} = "{{ env.1 }}"
{% endfor %}
[tasks.dev]
description = "Start development server"
run = "echo 'Development server starting...'"

{% for task in tasks -%}
[tasks.{{ task }}]
description = "{{ task | title }} task"
run = "echo 'Running {{ task }}...'"

{% endfor -%}
"#;

const PYTHON_PYPROJECT_TEMPLATE: &str = r#"[project]
name = "{{ project_name }}"
version = "0.1.0"
description = "{{ description }}"
requires-python = ">=3.12"
dependencies = [
{% for dep in dependencies -%}
    "{{ dep }}",
{% endfor -%}
]

[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"

[tool.hatch.build.targets.wheel]
packages = ["src/{{ project_name | lower | replace(from='-', to='_') }}"]

[tool.uv]
dev-dependencies = [
    "pytest>=8.0.0",
    "pytest-asyncio>=0.23.0",
    "ruff>=0.1.0",
]
"#;

const REACT_PACKAGE_JSON_TEMPLATE: &str = r#"{
  "name": "{{ project_name | lower }}",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "description": "{{ description }}",
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "preview": "vite preview",
    "lint": "eslint . --ext ts,tsx --report-unused-disable-directives --max-warnings 0"
  },
  "dependencies": {
    "react": "^18.3.1",
    "react-dom": "^18.3.1"
  },
  "devDependencies": {
    "@types/react": "^18.3.3",
    "@types/react-dom": "^18.3.0",
    "@typescript-eslint/eslint-plugin": "^7.13.1",
    "@typescript-eslint/parser": "^7.13.1",
    "@vitejs/plugin-react": "^4.3.1",
    "autoprefixer": "^10.4.19",
    "eslint": "^8.57.0",
    "eslint-plugin-react-hooks": "^4.6.2",
    "eslint-plugin-react-refresh": "^0.4.7",
    "postcss": "^8.4.38",
    "tailwindcss": "^3.4.4",
    "typescript": "^5.5.3",
    "vite": "^5.3.1"
  }
}
"#;

const REACT_TSCONFIG_TEMPLATE: &str = r#"{
  "compilerOptions": {
    "target": "ES2020",
    "useDefineForClassFields": true,
    "lib": ["ES2020", "DOM", "DOM.Iterable"],
    "module": "ESNext",
    "skipLibCheck": true,
    "moduleResolution": "bundler",
    "allowImportingTsExtensions": true,
    "resolveJsonModule": true,
    "isolatedModules": true,
    "noEmit": true,
    "jsx": "react-jsx",
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noFallthroughCasesInSwitch": true
  },
  "include": ["src"],
  "references": [{ "path": "./tsconfig.node.json" }]
}
"#;

const REACT_VITE_CONFIG_TEMPLATE: &str = r#"import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

export default defineConfig({
  plugins: [react()],
  server: {
    port: 3000,
  },
})
"#;

const REACT_TAILWIND_CONFIG_TEMPLATE: &str = r#"/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {},
  },
  plugins: [],
}
"#;

const DOCKER_COMPOSE_TEMPLATE: &str = r#"services:
{% for db in databases -%}
{% if db is object and db.Postgres -%}
  postgres:
    image: postgres:16
    restart: unless-stopped
    environment:
      POSTGRES_USER: {{ db.Postgres.user }}
      POSTGRES_PASSWORD: {{ db.Postgres.password }}
      POSTGRES_DB: {{ project_name | lower }}
    extra_hosts:
      - "host.docker.internal:{{ db.Postgres.host }}"
    ports:
      - "{{ db.Postgres.port }}:5432"

{% endif -%}
{% if db is object and db.Redis -%}
  redis:
    image: redis:7-alpine
    restart: unless-stopped
    extra_hosts:
      - "host.docker.internal:{{ db.Redis.host }}"
    ports:
      - "{{ db.Redis.port }}:6379"

{% endif -%}
{% if db is object and db.Qdrant -%}
  qdrant:
    image: qdrant/qdrant:latest
    restart: unless-stopped
    environment:
      QDRANT__SERVICE__HTTP_PORT: 6333
    ports:
      - "6333:6333"
    volumes:
      - qdrant_storage:/qdrant/storage

{% endif -%}
{% endfor -%}
{% if databases | length > 0 -%}
volumes:
{% for db in databases -%}
{% if db is object and db.Qdrant -%}
  qdrant_storage:
{% endif -%}
{% endfor -%}
{% endif -%}
"#;

const README_TEMPLATE: &str = r#"# {{ project_name }}

{{ description }}

## Stack

{% if stack is object and stack.PythonFastAPI -%}
- **Backend**: FastAPI (Python 3.12+)
- **Package Management**: UV
- **Build System**: Hatchling
{% endif -%}
{% if stack is object and stack.ReactVite -%}
- **Frontend**: React 18 + TypeScript
- **Build Tool**: Vite
- **Styling**: Tailwind CSS
- **Package Manager**: Bun
{% endif -%}

## Getting Started

### Prerequisites

- [mise](https://mise.jdx.dev/) for tool version management

### Installation

```bash
# Install tools and dependencies
mise install

{% if stack is object and stack.PythonFastAPI -%}
# Install Python dependencies
uv sync
{% endif -%}
{% if stack is object and stack.ReactVite -%}
# Install JavaScript dependencies
bun install
{% endif -%}
{% if has_databases -%}

# Start services
docker compose up -d
{% endif -%}
```

### Development

```bash
# Start development server
mise run dev
```

### Available Tasks

{% for task in mise_tasks -%}
- `mise run {{ task }}` - {{ task | title }} task
{% endfor %}

## Project Structure

{% if stack is object and stack.PythonFastAPI -%}
```
├── src/
│   └── {{ project_name | lower | replace(from='-', to='_') }}/
├── tests/
├── mise.toml
├── pyproject.toml
{% if has_databases -%}
├── compose.yml
{% endif -%}
└── README.md
```
{% endif -%}
{% if stack is object and stack.ReactVite -%}
```
├── src/
├── public/
├── mise.toml
├── package.json
├── vite.config.ts
├── tailwind.config.js
{% if has_databases -%}
├── compose.yml
{% endif -%}
└── README.md
```
{% endif -%}

## License

MIT
"#;
