-- School setup draft storage (one JSONB document per organization)
CREATE TABLE IF NOT EXISTS school_setups (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id      UUID NOT NULL UNIQUE REFERENCES organizations(id) ON DELETE CASCADE,
    data        JSONB NOT NULL DEFAULT '{}',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Reuse existing trigger function from users migration
CREATE TRIGGER update_school_setups_updated_at
    BEFORE UPDATE ON school_setups
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
