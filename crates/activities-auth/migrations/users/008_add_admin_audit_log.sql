-- Append-only audit trail of admin actions.
--
-- There is deliberately no foreign key to t_users on either the actor or the target:
--   * actors may be the break-glass CLI or the startup bootstrap, which have no user account;
--   * an audit trail must survive the deletion of the user it refers to.
-- Identifiers are therefore denormalized snapshots. Rows are never updated or deleted by the
-- application.
CREATE TABLE t_admin_audit_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    occurred_at TIMESTAMP NOT NULL,
    origin TEXT NOT NULL,
    actor_user_id TEXT,
    action TEXT NOT NULL,
    target_user_id TEXT,
    target_email TEXT,
    outcome TEXT NOT NULL,
    details TEXT
);

CREATE INDEX idx_admin_audit_occurred_at ON t_admin_audit_log (occurred_at);

CREATE INDEX idx_admin_audit_actor ON t_admin_audit_log (actor_user_id);

CREATE INDEX idx_admin_audit_target ON t_admin_audit_log (target_user_id);

CREATE INDEX idx_admin_audit_action ON t_admin_audit_log (action);
