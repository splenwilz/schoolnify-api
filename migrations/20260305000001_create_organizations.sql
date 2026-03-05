-- Organizations table (local mirror of WorkOS orgs, one per school)
CREATE TABLE IF NOT EXISTS organizations (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workos_org_id   TEXT NOT NULL UNIQUE,
    name            TEXT NOT NULL,
    slug            TEXT NOT NULL UNIQUE,
    domain          TEXT,
    is_active       BOOLEAN NOT NULL DEFAULT TRUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_organizations_workos_org_id ON organizations (workos_org_id);
CREATE INDEX idx_organizations_slug ON organizations (slug);

-- Reuse existing trigger function from users migration
CREATE TRIGGER update_organizations_updated_at
    BEFORE UPDATE ON organizations
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Add org_id to users (nullable for existing users)
ALTER TABLE users ADD COLUMN org_id UUID REFERENCES organizations(id);
CREATE INDEX idx_users_org_id ON users (org_id);
