-- Task Tracker Database Schema v2 migration
-- Removes allowed_approvers, adds maintainers table.

DROP INDEX IF EXISTS idx_allowed_approvers_repository_id;
DROP INDEX IF EXISTS idx_allowed_approvers_name;
DROP TABLE allowed_approvers;

CREATE TABLE maintainers (
    id SERIAL PRIMARY KEY,
    username VARCHAR(255) NOT NULL,
    domain VARCHAR(255) NOT NULL,
    review_score REAL NOT NULL DEFAULT 1.0,
    UNIQUE(username, domain)
);

UPDATE global SET schema_version = 2;
