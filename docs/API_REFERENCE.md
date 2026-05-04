# API Reference

The API documentation has been split into separate files for maintainability.

## Endpoint Docs

| File | Endpoints | Description |
|------|-----------|-------------|
| [api/README.md](api/README.md) | — | Overview, authentication, error format |
| [api/auth.md](api/auth.md) | `/api/v1/auth/*` | Signup, login, logout, session management, OAuth |
| [api/schools.md](api/schools.md) | `/api/v1/schools/*` | School setup wizard, public branding |
| [api/students.md](api/students.md) | `/api/v1/students/*` | Student CRUD, status/class changes, promotion, CSV import/export |
| [api/health.md](api/health.md) | `/health` | Health check |
| [api/types.md](api/types.md) | — | Shared response types (UserResponse, AuthResponse, etc.) |

## Integration Guides

| File | Description |
|------|-------------|
| [FRONTEND_INTEGRATION.md](FRONTEND_INTEGRATION.md) | Next.js integration, auth flows, subdomain routing |
| [SCHOOL_SETUP.md](SCHOOL_SETUP.md) | Setup wizard: 12 sections, auto-save, presets, merge behavior |
| [CONFIGURATION.md](CONFIGURATION.md) | Environment variables, dev/prod profiles |
| [ARCHITECTURE.md](ARCHITECTURE.md) | Tech stack, project structure, auth flow diagrams |
| [DATABASE_SCHEMA.md](DATABASE_SCHEMA.md) | Tables, indexes, migrations, ER diagram |

## Interactive Docs

Swagger UI is available at `http://localhost:8080/docs` when the server is running.
