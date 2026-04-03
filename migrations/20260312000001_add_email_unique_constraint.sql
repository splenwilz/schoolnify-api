-- Add UNIQUE constraint on users.email for defense-in-depth.
-- WorkOS already enforces email uniqueness, but the DB should guard against
-- manual inserts or migration bugs.

-- First, clean up any duplicate emails by keeping only the most recently created
-- record for each email. Duplicates arise when a user is deleted from WorkOS
-- and re-created, producing a new workos_user_id for the same email.
DELETE FROM users
WHERE id NOT IN (
    SELECT DISTINCT ON (email) id
    FROM users
    ORDER BY email, created_at DESC
);

-- Replace the existing non-unique index with a unique constraint.
DROP INDEX IF EXISTS idx_users_email;
ALTER TABLE users ADD CONSTRAINT users_email_unique UNIQUE (email);
