-- Admission number auto-generation config on school_configs.
-- Pattern: {prefix}/{year}/{seq:03}. Sequence resets per year.

ALTER TABLE school_configs
    ADD COLUMN IF NOT EXISTS admission_number_prefix    TEXT,
    ADD COLUMN IF NOT EXISTS admission_number_seq_year  SMALLINT,
    ADD COLUMN IF NOT EXISTS admission_number_next_seq  INTEGER NOT NULL DEFAULT 1;
