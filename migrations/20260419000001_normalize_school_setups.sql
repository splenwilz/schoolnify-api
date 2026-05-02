-- Normalize school_setups JSONB into relational tables.
-- Creates: school_configs (parent) + 8 child tables.
-- Migrates existing data, then drops the old school_setups table.

-- ── Parent: school_configs (one row per org, ~50 typed columns) ────────

CREATE TABLE IF NOT EXISTS school_configs (
    id                              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id                          UUID NOT NULL UNIQUE REFERENCES organizations(id) ON DELETE CASCADE,

    -- identity
    school_type                     TEXT,
    ownership_type                  TEXT,
    motto                           TEXT,
    founded_year                    TEXT,
    accreditation_number            TEXT,
    logo_url                        TEXT,

    -- branding
    primary_color                   TEXT,
    secondary_color                 TEXT,

    -- location
    country                         TEXT,
    state_region                    TEXT,
    city                            TEXT,
    timezone                        TEXT,

    -- localization
    currency                        TEXT,
    date_format                     TEXT,
    language                        TEXT,

    -- academic_calendar (scalars only; terms are in child table)
    calendar_type                   TEXT,
    current_academic_year           TEXT,

    -- grade_levels (scalars; individual levels are in child table)
    grade_level_structure_id        TEXT,
    group_sections                  JSONB NOT NULL DEFAULT '{}',
    custom_group_levels             JSONB NOT NULL DEFAULT '{}',

    -- grading (scalars; scale rows are in child table)
    grading_preset_id               TEXT,
    ca_weight                       TEXT,
    exam_weight                     TEXT,
    passmark                        TEXT,
    gpa_enabled                     BOOLEAN,
    assignment_weight               TEXT,
    test_weight                     TEXT,
    project_weight                  TEXT,

    -- subjects (scalars; individual subjects are in child table)
    subject_departments             JSONB NOT NULL DEFAULT '{}',

    -- fees (scalars; categories/discounts are in child tables)
    fee_payment_schedule            TEXT,
    fee_payment_due_day             TEXT,
    late_fee_percentage             TEXT,
    late_fee_grace_days             TEXT,

    -- report_card
    report_template                 TEXT,
    show_assessment_breakdown       BOOLEAN,
    show_class_average              BOOLEAN,
    show_highest_lowest             BOOLEAN,
    show_grading_legend             BOOLEAN,
    show_position                   BOOLEAN,
    show_gpa                        BOOLEAN,
    show_effort_grades              BOOLEAN,
    show_behavior_rating            BOOLEAN,
    show_psychomotor                BOOLEAN,
    psychomotor_traits              JSONB NOT NULL DEFAULT '[]',
    show_affective                  BOOLEAN,
    affective_traits                JSONB NOT NULL DEFAULT '[]',
    show_teacher_comments           BOOLEAN,
    show_class_teacher_comment      BOOLEAN,
    show_principal_signature        BOOLEAN,
    show_subject_teacher_signature  BOOLEAN,
    comment_char_limit              TEXT,
    show_attendance_summary         BOOLEAN,
    show_next_term_dates            BOOLEAN,
    show_co_curricular              BOOLEAN,

    -- policies
    attendance_tracking_methods     JSONB NOT NULL DEFAULT '{}',
    late_grace_period               TEXT,
    attendance_threshold            TEXT,
    tardies_to_absence              TEXT,
    consecutive_absence_alert       TEXT,
    absence_categories              JSONB NOT NULL DEFAULT '[]',
    promotion_criteria              TEXT,
    promotion_rules                 JSONB NOT NULL DEFAULT '{}',
    discipline_framework            TEXT,
    offense_categories              JSONB NOT NULL DEFAULT '[]',
    consequence_ladder              JSONB NOT NULL DEFAULT '[]',
    point_reset_period              TEXT,
    parent_portal                   BOOLEAN,
    report_comments                 BOOLEAN,
    attendance_alerts               BOOLEAN,
    fee_reminders                   BOOLEAN,
    exam_result_notify              BOOLEAN,
    behavior_alerts                 BOOLEAN,
    homework_alerts                 BOOLEAN,
    notification_channels           JSONB NOT NULL DEFAULT '[]',

    created_at                      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at                      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TRIGGER update_school_configs_updated_at
    BEFORE UPDATE ON school_configs FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- ── Child tables ───────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS school_grading_scales (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id      UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    grade       TEXT NOT NULL,
    min_score   TEXT NOT NULL,
    max_score   TEXT NOT NULL,
    descriptor  TEXT,
    gpa_points  TEXT,
    position    SMALLINT NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_school_grading_scales_org_id ON school_grading_scales(org_id);
CREATE TRIGGER update_school_grading_scales_updated_at
    BEFORE UPDATE ON school_grading_scales FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TABLE IF NOT EXISTS school_terms (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id      UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    start_date  TEXT,
    end_date    TEXT,
    position    SMALLINT NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_school_terms_org_id ON school_terms(org_id);
CREATE TRIGGER update_school_terms_updated_at
    BEFORE UPDATE ON school_terms FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TABLE IF NOT EXISTS school_subjects (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id      UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    department  TEXT,
    position    SMALLINT NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_school_subjects_org_id ON school_subjects(org_id);
CREATE TRIGGER update_school_subjects_updated_at
    BEFORE UPDATE ON school_subjects FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TABLE IF NOT EXISTS school_grade_levels (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id      UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    group_name  TEXT,
    position    SMALLINT NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_school_grade_levels_org_id ON school_grade_levels(org_id);
CREATE TRIGGER update_school_grade_levels_updated_at
    BEFORE UPDATE ON school_grade_levels FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TABLE IF NOT EXISTS school_fee_categories (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id      UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    mandatory   BOOLEAN NOT NULL DEFAULT false,
    frequency   TEXT,
    fee_type    TEXT,
    applies_to  TEXT,
    grade_levels JSONB NOT NULL DEFAULT '[]',
    amounts     JSONB NOT NULL DEFAULT '{}',
    position    SMALLINT NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_school_fee_categories_org_id ON school_fee_categories(org_id);
CREATE TRIGGER update_school_fee_categories_updated_at
    BEFORE UPDATE ON school_fee_categories FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TABLE IF NOT EXISTS school_fee_discounts (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id      UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    percentage  TEXT,
    applies_to  TEXT,
    position    SMALLINT NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_school_fee_discounts_org_id ON school_fee_discounts(org_id);
CREATE TRIGGER update_school_fee_discounts_updated_at
    BEFORE UPDATE ON school_fee_discounts FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TABLE IF NOT EXISTS school_schedule_groups (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id          UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    group_name      TEXT NOT NULL,
    start_time      TEXT,
    end_time        TEXT,
    period_duration TEXT,
    position        SMALLINT NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_school_schedule_groups_org_id ON school_schedule_groups(org_id);
CREATE TRIGGER update_school_schedule_groups_updated_at
    BEFORE UPDATE ON school_schedule_groups FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TABLE IF NOT EXISTS school_schedule_periods (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    group_id    UUID NOT NULL REFERENCES school_schedule_groups(id) ON DELETE CASCADE,
    label       TEXT NOT NULL,
    start_time  TEXT,
    end_time    TEXT,
    is_break    BOOLEAN NOT NULL DEFAULT false,
    position    SMALLINT NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_school_schedule_periods_group_id ON school_schedule_periods(group_id);
CREATE TRIGGER update_school_schedule_periods_updated_at
    BEFORE UPDATE ON school_schedule_periods FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- ── Migrate existing JSONB data ────────────────────────────────────────

-- Populate school_configs from school_setups.data
INSERT INTO school_configs (
    org_id,
    school_type, ownership_type, motto, founded_year, accreditation_number, logo_url,
    primary_color, secondary_color,
    country, state_region, city, timezone,
    currency, date_format, language,
    calendar_type, current_academic_year,
    grade_level_structure_id, group_sections, custom_group_levels,
    grading_preset_id, ca_weight, exam_weight, passmark,
    gpa_enabled, assignment_weight, test_weight, project_weight,
    subject_departments,
    fee_payment_schedule, fee_payment_due_day, late_fee_percentage, late_fee_grace_days,
    report_template, show_position, show_gpa, show_teacher_comments,
    show_principal_signature, show_attendance_summary, show_behavior_rating,
    show_subject_teacher_signature, show_assessment_breakdown, show_class_average,
    show_highest_lowest, show_grading_legend, show_effort_grades, show_psychomotor,
    psychomotor_traits, show_affective, affective_traits, show_class_teacher_comment,
    comment_char_limit, show_next_term_dates, show_co_curricular,
    attendance_tracking_methods, late_grace_period, attendance_threshold,
    tardies_to_absence, consecutive_absence_alert, absence_categories,
    promotion_criteria, promotion_rules,
    discipline_framework, offense_categories, consequence_ladder, point_reset_period,
    parent_portal, report_comments, attendance_alerts, fee_reminders,
    exam_result_notify, behavior_alerts, homework_alerts, notification_channels
)
SELECT
    org_id,
    data->'identity'->>'school_type',
    data->'identity'->>'ownership_type',
    data->'identity'->>'motto',
    data->'identity'->>'founded_year',
    data->'identity'->>'accreditation_number',
    COALESCE(data->'branding'->>'logo_url', data->'identity'->>'logo_url'),
    data->'branding'->>'primary_color',
    data->'branding'->>'secondary_color',
    data->'location'->>'country',
    data->'location'->>'state_region',
    data->'location'->>'city',
    data->'location'->>'timezone',
    data->'localization'->>'currency',
    data->'localization'->>'date_format',
    data->'localization'->>'language',
    data->'academic_calendar'->>'calendar_type',
    data->'academic_calendar'->>'current_academic_year',
    data->'grade_levels'->>'grade_level_structure_id',
    COALESCE(data->'grade_levels'->'group_sections', '{}'),
    COALESCE(data->'grade_levels'->'custom_group_levels', '{}'),
    data->'grading'->>'grading_preset_id',
    data->'grading'->>'ca_weight',
    data->'grading'->>'exam_weight',
    data->'grading'->>'passmark',
    (data->'grading'->>'gpa_enabled')::boolean,
    data->'grading'->>'assignment_weight',
    data->'grading'->>'test_weight',
    data->'grading'->>'project_weight',
    COALESCE(data->'subjects'->'subject_departments', '{}'),
    data->'fees'->>'fee_payment_schedule',
    data->'fees'->>'fee_payment_due_day',
    data->'fees'->>'late_fee_percentage',
    data->'fees'->>'late_fee_grace_days',
    data->'report_card'->>'report_template',
    (data->'report_card'->>'show_position')::boolean,
    (data->'report_card'->>'show_gpa')::boolean,
    (data->'report_card'->>'show_teacher_comments')::boolean,
    (data->'report_card'->>'show_principal_signature')::boolean,
    (data->'report_card'->>'show_attendance_summary')::boolean,
    (data->'report_card'->>'show_behavior_rating')::boolean,
    (data->'report_card'->>'show_subject_teacher_signature')::boolean,
    (data->'report_card'->>'show_assessment_breakdown')::boolean,
    (data->'report_card'->>'show_class_average')::boolean,
    (data->'report_card'->>'show_highest_lowest')::boolean,
    (data->'report_card'->>'show_grading_legend')::boolean,
    (data->'report_card'->>'show_effort_grades')::boolean,
    (data->'report_card'->>'show_psychomotor')::boolean,
    COALESCE(data->'report_card'->'psychomotor_traits', '[]'),
    (data->'report_card'->>'show_affective')::boolean,
    COALESCE(data->'report_card'->'affective_traits', '[]'),
    (data->'report_card'->>'show_class_teacher_comment')::boolean,
    data->'report_card'->>'comment_char_limit',
    (data->'report_card'->>'show_next_term_dates')::boolean,
    (data->'report_card'->>'show_co_curricular')::boolean,
    COALESCE(data->'policies'->'attendance_tracking_methods', '{}'),
    data->'policies'->>'late_grace_period',
    data->'policies'->>'attendance_threshold',
    data->'policies'->>'tardies_to_absence',
    data->'policies'->>'consecutive_absence_alert',
    COALESCE(data->'policies'->'absence_categories', '[]'),
    data->'policies'->>'promotion_criteria',
    COALESCE(data->'policies'->'promotion_rules', '{}'),
    data->'policies'->>'discipline_framework',
    COALESCE(data->'policies'->'offense_categories', '[]'),
    COALESCE(data->'policies'->'consequence_ladder', '[]'),
    data->'policies'->>'point_reset_period',
    (data->'policies'->>'parent_portal')::boolean,
    (data->'policies'->>'report_comments')::boolean,
    (data->'policies'->>'attendance_alerts')::boolean,
    (data->'policies'->>'fee_reminders')::boolean,
    (data->'policies'->>'exam_result_notify')::boolean,
    (data->'policies'->>'behavior_alerts')::boolean,
    (data->'policies'->>'homework_alerts')::boolean,
    COALESCE(data->'policies'->'notification_channels', '[]')
FROM school_setups
WHERE data IS NOT NULL AND data != '{}'::jsonb;

-- Populate grading scales
INSERT INTO school_grading_scales (org_id, grade, min_score, max_score, descriptor, gpa_points, position)
SELECT
    s.org_id,
    elem->>'grade',
    COALESCE(elem->>'min_score', '0'),
    COALESCE(elem->>'max_score', '0'),
    elem->>'descriptor',
    elem->>'gpa_points',
    (row_number() OVER (PARTITION BY s.org_id ORDER BY ordinality))::smallint - 1
FROM school_setups s,
     jsonb_array_elements(s.data->'grading'->'grading_scale') WITH ORDINALITY AS t(elem, ordinality)
WHERE jsonb_typeof(COALESCE(s.data->'grading'->'grading_scale', 'null')) = 'array';

-- Populate terms
INSERT INTO school_terms (org_id, name, start_date, end_date, position)
SELECT
    s.org_id,
    COALESCE(elem->>'name', ''),
    elem->>'start_date',
    elem->>'end_date',
    (row_number() OVER (PARTITION BY s.org_id ORDER BY ordinality))::smallint - 1
FROM school_setups s,
     jsonb_array_elements(s.data->'academic_calendar'->'terms') WITH ORDINALITY AS t(elem, ordinality)
WHERE jsonb_typeof(COALESCE(s.data->'academic_calendar'->'terms', 'null')) = 'array';

-- Populate subjects (from string array + department map)
INSERT INTO school_subjects (org_id, name, department, position)
SELECT
    s.org_id,
    elem::text,
    s.data->'subjects'->'subject_departments'->>elem::text,
    (row_number() OVER (PARTITION BY s.org_id ORDER BY ordinality))::smallint - 1
FROM school_setups s,
     jsonb_array_elements_text(s.data->'subjects'->'subjects') WITH ORDINALITY AS t(elem, ordinality)
WHERE jsonb_typeof(COALESCE(s.data->'subjects'->'subjects', 'null')) = 'array';

-- Populate grade levels (from string array)
INSERT INTO school_grade_levels (org_id, name, position)
SELECT
    s.org_id,
    elem::text,
    (row_number() OVER (PARTITION BY s.org_id ORDER BY ordinality))::smallint - 1
FROM school_setups s,
     jsonb_array_elements_text(s.data->'grade_levels'->'grade_levels') WITH ORDINALITY AS t(elem, ordinality)
WHERE jsonb_typeof(COALESCE(s.data->'grade_levels'->'grade_levels', 'null')) = 'array';

-- Populate fee categories
INSERT INTO school_fee_categories (org_id, name, mandatory, frequency, fee_type, applies_to, grade_levels, amounts, position)
SELECT
    s.org_id,
    COALESCE(elem->>'name', ''),
    COALESCE((elem->>'mandatory')::boolean, false),
    elem->>'frequency',
    elem->>'fee_type',
    elem->>'applies_to',
    COALESCE(elem->'grade_levels', '[]'),
    COALESCE(elem->'amounts', '{}'),
    (row_number() OVER (PARTITION BY s.org_id ORDER BY ordinality))::smallint - 1
FROM school_setups s,
     jsonb_array_elements(s.data->'fees'->'fee_categories') WITH ORDINALITY AS t(elem, ordinality)
WHERE jsonb_typeof(COALESCE(s.data->'fees'->'fee_categories', 'null')) = 'array';

-- Populate fee discounts
INSERT INTO school_fee_discounts (org_id, name, percentage, applies_to, position)
SELECT
    s.org_id,
    COALESCE(elem->>'name', ''),
    elem->>'percentage',
    elem->>'applies_to',
    (row_number() OVER (PARTITION BY s.org_id ORDER BY ordinality))::smallint - 1
FROM school_setups s,
     jsonb_array_elements(s.data->'fees'->'discount_types') WITH ORDINALITY AS t(elem, ordinality)
WHERE jsonb_typeof(COALESCE(s.data->'fees'->'discount_types', 'null')) = 'array';

-- Note: schedule groups/periods migration is complex (keyed objects).
-- Handled via a DO block:
DO $$
DECLARE
    r RECORD;
    group_key TEXT;
    group_val JSONB;
    group_uuid UUID;
    period_val JSONB;
    group_pos INT;
    period_pos INT;
BEGIN
    FOR r IN SELECT org_id, data->'schedule'->'schedules' AS schedules FROM school_setups
             WHERE jsonb_typeof(COALESCE(data->'schedule'->'schedules', 'null')) = 'object'
    LOOP
        group_pos := 0;
        FOR group_key, group_val IN SELECT * FROM jsonb_each(r.schedules) LOOP
            INSERT INTO school_schedule_groups (org_id, group_name, start_time, end_time, period_duration, position)
            VALUES (r.org_id, group_key, group_val->>'start_time', group_val->>'end_time', group_val->>'period_duration', group_pos)
            RETURNING id INTO group_uuid;

            IF jsonb_typeof(COALESCE(group_val->'periods', 'null')) = 'array' THEN
                period_pos := 0;
                FOR period_val IN SELECT * FROM jsonb_array_elements(group_val->'periods') LOOP
                    INSERT INTO school_schedule_periods (group_id, label, start_time, end_time, is_break, position)
                    VALUES (group_uuid, COALESCE(period_val->>'label', ''), period_val->>'start_time', period_val->>'end_time', COALESCE((period_val->>'is_break')::boolean, false), period_pos);
                    period_pos := period_pos + 1;
                END LOOP;
            END IF;

            group_pos := group_pos + 1;
        END LOOP;
    END LOOP;
END $$;

-- ── Drop old table ─────────────────────────────────────────────────────

DROP TABLE IF EXISTS school_setups;
