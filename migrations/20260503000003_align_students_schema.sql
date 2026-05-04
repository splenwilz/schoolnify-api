-- Forward migration: bring student tables to the design used by the current
-- service layer.
--
-- 20260503000001 created the initial student schema with simple per-table FKs,
-- one-way status/date consistency CHECKs, no stream tracking on the class-
-- history audit, and no enum CHECKs on status-history columns. Service-layer
-- iteration tightened those guarantees; this migration applies the equivalent
-- ALTERs without dropping data.

-- 1. Allow composite (student_id, org_id) FKs from child tables.
ALTER TABLE students
    ADD CONSTRAINT students_id_org_unique UNIQUE (id, org_id);

-- 2. Bidirectional consistency on students:
--      graduation_date IS NOT NULL  iff  status = 'graduated'
--      withdrawn_at    IS NOT NULL  iff  status = 'withdrawn'
-- The full timeline lives in student_status_history; the row only carries the
-- date that matches the current state. Clear any stale dates before the swap.
UPDATE students SET graduation_date = NULL WHERE status <> 'graduated';
UPDATE students SET withdrawn_at = NULL    WHERE status <> 'withdrawn';

ALTER TABLE students
    DROP CONSTRAINT students_graduation_consistency_chk,
    ADD  CONSTRAINT students_graduation_consistency_chk CHECK (
        (status = 'graduated' AND graduation_date IS NOT NULL)
        OR (status <> 'graduated' AND graduation_date IS NULL)
    );

ALTER TABLE students
    DROP CONSTRAINT students_withdrawn_consistency_chk,
    ADD  CONSTRAINT students_withdrawn_consistency_chk CHECK (
        (status = 'withdrawn' AND withdrawn_at IS NOT NULL)
        OR (status <> 'withdrawn' AND withdrawn_at IS NULL)
    );

-- 3. Replace simple student_id FKs with composite (student_id, org_id) FKs so
--    the DB rejects any audit/guardian row whose org_id doesn't match its
--    student's org_id.
ALTER TABLE student_guardians
    DROP CONSTRAINT student_guardians_student_id_fkey,
    ADD  CONSTRAINT student_guardians_student_org_fk
        FOREIGN KEY (student_id, org_id) REFERENCES students(id, org_id) ON DELETE CASCADE;

ALTER TABLE student_status_history
    DROP CONSTRAINT student_status_history_student_id_fkey,
    ADD  CONSTRAINT student_status_history_student_org_fk
        FOREIGN KEY (student_id, org_id) REFERENCES students(id, org_id) ON DELETE CASCADE;

ALTER TABLE student_class_history
    DROP CONSTRAINT student_class_history_student_id_fkey,
    ADD  CONSTRAINT student_class_history_student_org_fk
        FOREIGN KEY (student_id, org_id) REFERENCES students(id, org_id) ON DELETE CASCADE;

-- 4. Constrain audit status fields to the same enum as students.status.
ALTER TABLE student_status_history
    ADD CONSTRAINT student_status_history_from_chk
        CHECK (from_status IN ('active', 'inactive', 'suspended', 'graduated', 'withdrawn', 'transferred')),
    ADD CONSTRAINT student_status_history_to_chk
        CHECK (to_status IN ('active', 'inactive', 'suspended', 'graduated', 'withdrawn', 'transferred'));

-- 5. Track stream changes in the class-history audit so stream-only edits and
--    promotions that change stream are captured.
ALTER TABLE student_class_history
    ADD COLUMN from_stream TEXT,
    ADD COLUMN to_stream   TEXT;
