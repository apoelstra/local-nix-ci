CREATE SCHEMA local_ci;

CREATE TABLE local_ci.global (
    schema_version INTEGER NOT NULL
);
INSERT INTO local_ci.global (schema_version)
VALUES (1);
