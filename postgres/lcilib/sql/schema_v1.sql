-- Task Tracker Database Schema

CREATE TYPE ack_status AS ENUM ('pending', 'failed', 'posted', 'external');
CREATE TYPE ci_status AS ENUM ('unstarted', 'skipped', 'failed', 'passed');
CREATE TYPE merge_status AS ENUM ('pending', 'cancelled', 'failed', 'pushed');
CREATE TYPE review_status AS ENUM ('unreviewed', 'rejected', 'approved');

CREATE TYPE commit_type AS ENUM ('normal', 'single', 'tip', 'merge');
CREATE TYPE entity_type AS ENUM ('commit', 'pull_request', 'stack', 'ack', 'system'); -- for logs

-- Version
CREATE TABLE global (
    schema_version INTEGER NOT NULL
);
INSERT INTO global (schema_version)
VALUES (1);

-- Repositories table to normalize repository information
CREATE TABLE repositories (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    path TEXT NOT NULL UNIQUE,
    nixfile_path TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Commits table for git commits with jj change IDs
CREATE TABLE commits (
    id SERIAL PRIMARY KEY,
    repository_id INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    git_commit_id VARCHAR(64) NOT NULL, -- SHA-1 or maybe SHA-256 hash
    jj_change_id VARCHAR(64) NOT NULL,
    review_status review_status NOT NULL DEFAULT 'unreviewed',
    should_run_ci BOOLEAN NOT NULL DEFAULT FALSE,
    ci_status ci_status NOT NULL DEFAULT 'unstarted',
    commit_type commit_type NOT NULL,
    nix_derivation TEXT, -- CI job derivation
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    
    UNIQUE(repository_id, git_commit_id),
    UNIQUE(repository_id, jj_change_id)
);

-- Pull requests table
CREATE TABLE pull_requests (
    id SERIAL PRIMARY KEY,
    repository_id INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    pr_number INTEGER NOT NULL,
    title TEXT NOT NULL DEFAULT '',
    body TEXT NOT NULL DEFAULT '',
    tip_commit_id INTEGER NOT NULL REFERENCES commits(id) ON DELETE CASCADE,
    review_status review_status NOT NULL DEFAULT 'unreviewed',
    priority INTEGER NOT NULL DEFAULT 0,
    ok_to_merge BOOLEAN NOT NULL DEFAULT TRUE,
    required_reviewers INTEGER NOT NULL DEFAULT 1,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    synced_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    
    UNIQUE(repository_id, pr_number)
);

-- Junction table for PR-commit relationships with ordering
CREATE TABLE pr_commits (
    id SERIAL PRIMARY KEY,
    pull_request_id INTEGER NOT NULL REFERENCES pull_requests(id) ON DELETE CASCADE,
    commit_id INTEGER NOT NULL REFERENCES commits(id) ON DELETE CASCADE,
    sequence_order INTEGER NOT NULL,
    
    UNIQUE(pull_request_id, commit_id),
    UNIQUE(pull_request_id, sequence_order)
);

-- Stacks table for merge commits ready to be pushed
CREATE TABLE stacks (
    id SERIAL PRIMARY KEY,
    repository_id INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    target_branch VARCHAR(255) NOT NULL,
    status merge_status NOT NULL DEFAULT 'pending',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Junction table for stack-commit relationships with ordering
CREATE TABLE stack_commits (
    id SERIAL PRIMARY KEY,
    stack_id INTEGER NOT NULL REFERENCES stacks(id) ON DELETE CASCADE,
    commit_id INTEGER NOT NULL REFERENCES commits(id) ON DELETE CASCADE,
    sequence_order INTEGER NOT NULL,
    
    UNIQUE(stack_id, commit_id),
    UNIQUE(stack_id, sequence_order)
);

-- ACKs table for tracking reviewer acknowledgments
CREATE TABLE acks (
    id SERIAL PRIMARY KEY,
    pull_request_id INTEGER NOT NULL REFERENCES pull_requests(id) ON DELETE CASCADE,
    commit_id INTEGER NOT NULL REFERENCES commits(id) ON DELETE CASCADE, -- which commit this ACK is for
    reviewer_name VARCHAR(255) NOT NULL,
    message TEXT NOT NULL,
    status ack_status NOT NULL DEFAULT 'pending',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    
    UNIQUE(pull_request_id, reviewer_name)
);

-- Allowed approvers table for tracking who can approve PRs
CREATE TABLE allowed_approvers (
    id SERIAL PRIMARY KEY,
    repository_id INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    approver_name VARCHAR(255) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    
    UNIQUE(repository_id, approver_name)
);

-- Polymorphic log table for all actions
CREATE TABLE logs (
    id SERIAL PRIMARY KEY,
    entity_type entity_type NOT NULL,
    entity_id INTEGER NOT NULL,
    action VARCHAR(100) NOT NULL,
    description TEXT,
    reason TEXT,
    timestamp TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for performance
CREATE INDEX idx_commits_repository_id ON commits(repository_id);
CREATE INDEX idx_commits_review_status ON commits(review_status);
CREATE INDEX idx_commits_ci_status ON commits(ci_status);
CREATE INDEX idx_commits_should_run_ci ON commits(should_run_ci);
CREATE INDEX idx_commits_commit_type ON commits(commit_type);

CREATE INDEX idx_pull_requests_repository_id ON pull_requests(repository_id);
CREATE INDEX idx_pull_requests_review_status ON pull_requests(review_status);
CREATE INDEX idx_pull_requests_ok_to_merge ON pull_requests(ok_to_merge);

CREATE INDEX idx_pr_commits_pull_request_id ON pr_commits(pull_request_id);
CREATE INDEX idx_pr_commits_commit_id ON pr_commits(commit_id);

CREATE INDEX idx_stacks_repository_id ON stacks(repository_id);
CREATE INDEX idx_stacks_status ON stacks(status);
CREATE INDEX idx_stacks_target_branch ON stacks(target_branch);
CREATE INDEX idx_stacks_repo_branch ON stacks(repository_id, target_branch);

CREATE INDEX idx_stack_commits_stack_id ON stack_commits(stack_id);
CREATE INDEX idx_stack_commits_commit_id ON stack_commits(commit_id);

CREATE INDEX idx_acks_pull_request_id ON acks(pull_request_id);
CREATE INDEX idx_acks_commit_id ON acks(commit_id);
CREATE INDEX idx_acks_status ON acks(status);

CREATE INDEX idx_allowed_approvers_repository_id ON allowed_approvers(repository_id);
CREATE INDEX idx_allowed_approvers_name ON allowed_approvers(approver_name);

CREATE INDEX idx_logs_entity_type_id ON logs(entity_type, entity_id);
CREATE INDEX idx_logs_timestamp ON logs(timestamp);

-- Triggers to automatically update updated_at timestamps
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER update_commits_updated_at BEFORE UPDATE ON commits
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_pull_requests_updated_at BEFORE UPDATE ON pull_requests
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_stacks_updated_at BEFORE UPDATE ON stacks
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_acks_updated_at BEFORE UPDATE ON acks
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Business logic triggers for commits
CREATE OR REPLACE FUNCTION enforce_commit_constraints()
RETURNS TRIGGER AS $$
BEGIN
    -- Reset CI status when commit type changes
    IF OLD.commit_type IS DISTINCT FROM NEW.commit_type THEN
        NEW.ci_status = 'unstarted';
    END IF;
    
    -- Prevent should_run_ci = false with ci_status = failed/passed
    IF NEW.should_run_ci = false AND NEW.ci_status IN ('failed', 'passed') THEN
        RAISE EXCEPTION 'Cannot have should_run_ci = false with ci_status = failed or passed';
    END IF;
    
    -- Ensure merge commits are approved
    IF NEW.commit_type = 'merge' AND NEW.review_status != 'approved' THEN
        RAISE EXCEPTION 'Merge commits must have review_status = approved';
    END IF;
    
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER enforce_commit_constraints_trigger BEFORE UPDATE ON commits
    FOR EACH ROW EXECUTE FUNCTION enforce_commit_constraints();

CREATE TRIGGER enforce_commit_constraints_insert_trigger BEFORE INSERT ON commits
    FOR EACH ROW EXECUTE FUNCTION enforce_commit_constraints();

-- Business logic triggers for ACKs
CREATE OR REPLACE FUNCTION enforce_ack_constraints()
RETURNS TRIGGER AS $$
BEGIN
    -- Reset status to pending when internal ACK message changes
    IF OLD.status = 'posted' AND OLD.message IS DISTINCT FROM NEW.message THEN
        NEW.status = 'pending';
    END IF;
    
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER enforce_ack_constraints_trigger BEFORE UPDATE ON acks
    FOR EACH ROW EXECUTE FUNCTION enforce_ack_constraints();

-- Business logic triggers for pull requests
CREATE OR REPLACE FUNCTION enforce_pr_constraints()
RETURNS TRIGGER AS $$
BEGIN
    -- Validate PR approval: all commits must be approved
    IF NEW.review_status = 'approved' THEN
        IF EXISTS (
            SELECT 1 FROM pr_commits pc 
            JOIN commits c ON pc.commit_id = c.id 
            WHERE pc.pull_request_id = NEW.id 
            AND c.review_status != 'approved'
        ) THEN
            RAISE EXCEPTION 'Cannot approve PR: not all commits are approved';
        END IF;
    END IF;
    
    -- Reset review status and cancel merge commits when tip changes
    IF OLD.tip_commit_id IS DISTINCT FROM NEW.tip_commit_id THEN
        NEW.review_status = 'unreviewed';
        
        -- Cancel related merge commits that aren't already failed/passed
        UPDATE commits SET ci_status = 'skipped' 
        WHERE commit_type = 'merge' 
        AND ci_status NOT IN ('failed', 'passed')
        AND id IN (
            SELECT sc.commit_id FROM stack_commits sc
            JOIN stacks s ON sc.stack_id = s.id
            WHERE s.repository_id = NEW.repository_id
        );
    END IF;
    
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER enforce_pr_constraints_trigger BEFORE UPDATE ON pull_requests
    FOR EACH ROW EXECUTE FUNCTION enforce_pr_constraints();

CREATE TRIGGER enforce_pr_constraints_insert_trigger BEFORE INSERT ON pull_requests
    FOR EACH ROW EXECUTE FUNCTION enforce_pr_constraints();
