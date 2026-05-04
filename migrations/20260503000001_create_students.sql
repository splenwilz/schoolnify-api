-- Students module: core student records, guardians, and audit history.
-- Soft-delete only (status = 'withdrawn'). All scoping via org_id.

-- ── students: main row ────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS students (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id              UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,

    -- Identity
    admission_number    TEXT NOT NULL,
    first_name          TEXT NOT NULL,
    middle_name         TEXT,
    last_name           TEXT NOT NULL,
    date_of_birth       DATE NOT NULL,
    gender              TEXT NOT NULL,

    -- Class
    grade_level         TEXT NOT NULL,
    section             TEXT,
    stream              TEXT,

    -- Enrollment
    enrollment_date     DATE NOT NULL DEFAULT CURRENT_DATE,
    status              TEXT NOT NULL DEFAULT 'active',
    boarding_status     TEXT,

    -- Contact
    phone               TEXT,
    email               TEXT,
    address             TEXT,
    city                TEXT,
    state               TEXT,
    postal_code         TEXT,

    -- Health
    blood_group         TEXT,
    genotype            TEXT,
    allergies           TEXT,
    medical_conditions  TEXT,

    -- Background
    previous_school     TEXT,
    state_of_origin     TEXT,
    lga                 TEXT,
    religion            TEXT,
    tribe               TEXT,

    -- Misc
    avatar_url          TEXT,
    graduation_date     DATE,
    withdrawn_at        TIMESTAMPTZ,

    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT students_admission_number_unique UNIQUE (org_id, admission_number),
    CONSTRAINT students_gender_chk CHECK (gender IN ('male', 'female')),
    CONSTRAINT students_status_chk CHECK (status IN ('active', 'inactive', 'suspended', 'graduated', 'withdrawn', 'transferred')),
    CONSTRAINT students_boarding_chk CHECK (boarding_status IS NULL OR boarding_status IN ('day', 'boarding', 'weekly_boarding')),
    CONSTRAINT students_graduation_consistency_chk CHECK (status <> 'graduated' OR graduation_date IS NOT NULL),
    CONSTRAINT students_withdrawn_consistency_chk CHECK (status <> 'withdrawn' OR withdrawn_at IS NOT NULL)
);

CREATE INDEX idx_students_org_id ON students(org_id);
CREATE INDEX idx_students_org_status ON students(org_id, status);
CREATE INDEX idx_students_org_grade_section ON students(org_id, grade_level, section);
CREATE INDEX idx_students_org_last_name ON students(org_id, last_name);
CREATE INDEX idx_students_org_enrollment_date ON students(org_id, enrollment_date DESC);

CREATE TRIGGER update_students_updated_at
    BEFORE UPDATE ON students FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- ── student_guardians: 1-to-many ─────────────────────────────────────

CREATE TABLE IF NOT EXISTS student_guardians (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    student_id      UUID NOT NULL REFERENCES students(id) ON DELETE CASCADE,
    org_id          UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,

    first_name      TEXT NOT NULL,
    last_name       TEXT NOT NULL,
    phone           TEXT,
    email           TEXT,
    relationship    TEXT,
    occupation      TEXT,
    is_primary      BOOLEAN NOT NULL DEFAULT FALSE,
    position        SMALLINT NOT NULL DEFAULT 0,

    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_student_guardians_student_id ON student_guardians(student_id);
CREATE INDEX idx_student_guardians_org_id ON student_guardians(org_id);
-- At most one primary guardian per student. "At least one" stays a service-layer invariant.
CREATE UNIQUE INDEX idx_student_guardians_one_primary
    ON student_guardians(student_id) WHERE is_primary;

CREATE TRIGGER update_student_guardians_updated_at
    BEFORE UPDATE ON student_guardians FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- ── student_status_history: audit log ────────────────────────────────

CREATE TABLE IF NOT EXISTS student_status_history (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    student_id          UUID NOT NULL REFERENCES students(id) ON DELETE CASCADE,
    org_id              UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,

    from_status         TEXT NOT NULL,
    to_status           TEXT NOT NULL,
    reason              TEXT,
    effective_date      DATE NOT NULL DEFAULT CURRENT_DATE,
    changed_by_user_id  UUID REFERENCES users(id) ON DELETE SET NULL,
    changed_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_student_status_history_student ON student_status_history(student_id, changed_at DESC);
CREATE INDEX idx_student_status_history_org ON student_status_history(org_id, changed_at DESC);

-- ── student_class_history: audit log + promotion batch ───────────────

CREATE TABLE IF NOT EXISTS student_class_history (
    id                      UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    student_id              UUID NOT NULL REFERENCES students(id) ON DELETE CASCADE,
    org_id                  UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,

    from_grade_level        TEXT,
    from_section            TEXT,
    to_grade_level          TEXT,
    to_section              TEXT,
    change_kind             TEXT NOT NULL,
    reason                  TEXT,
    effective_date          DATE NOT NULL DEFAULT CURRENT_DATE,
    changed_by_user_id      UUID REFERENCES users(id) ON DELETE SET NULL,
    promotion_batch_id      UUID,
    changed_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT student_class_history_kind_chk CHECK (change_kind IN ('promote', 'retain', 'graduate', 'manual'))
);

CREATE INDEX idx_student_class_history_student ON student_class_history(student_id, changed_at DESC);
CREATE INDEX idx_student_class_history_org ON student_class_history(org_id, changed_at DESC);
CREATE INDEX idx_student_class_history_batch ON student_class_history(promotion_batch_id) WHERE promotion_batch_id IS NOT NULL;
