//! Audit trail of admin actions.
//!
//! Every state-changing admin operation, whether it succeeded or was refused, produces one
//! [`AdminAuditEntry`]. The actor is not linked to a user through a foreign key: admin actions can
//! be performed by the break-glass CLI or the startup bootstrap (no user account), and the trail
//! must survive the deletion of the users it refers to. Identifiers are stored as snapshots.

use chrono::{DateTime, Utc};

use crate::domain::models::UserId;

use super::EmailAddress;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdminActor {
    Web(UserId),
    Cli,
    Bootstrap,
}

impl AdminActor {
    pub fn user(&self) -> Option<&UserId> {
        match self {
            Self::Web(user) => Some(user),
            Self::Cli | Self::Bootstrap => None,
        }
    }

    pub fn origin(&self) -> &str {
        match self {
            Self::Web(_) => "web",
            Self::Cli => "cli",
            Self::Bootstrap => "bootstrap",
        }
    }
}

/// The admin operation being audited.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdminAction {
    CreateUser,
    SetUserPassword,
    SetUserEmail,
    SetAdmin,
    RevokeUserSessions,
    BootstrapAdmin,
    PromoteAdmin,
    RevokeAdmin,
}

impl AdminAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::CreateUser => "create_user",
            Self::SetUserPassword => "set_user_password",
            Self::SetUserEmail => "set_user_email",
            Self::SetAdmin => "set_admin",
            Self::RevokeUserSessions => "revoke_user_sessions",
            Self::BootstrapAdmin => "bootstrap_admin",
            Self::PromoteAdmin => "promote_admin",
            Self::RevokeAdmin => "revoke_admin",
        }
    }
}

#[derive(Debug, Clone)]
pub struct AdminAuditContext {
    actor: AdminActor,
    occurred_at: DateTime<Utc>,
}

impl AdminAuditContext {
    pub fn new(actor: AdminActor, occurred_at: DateTime<Utc>) -> Self {
        Self { actor, occurred_at }
    }

    pub fn actor(&self) -> &AdminActor {
        &self.actor
    }

    pub fn occurred_at(&self) -> &DateTime<Utc> {
        &self.occurred_at
    }

    /// Builds an audit entry for this actor/timestamp. `details`, when present, must never contain
    /// secrets (passwords, password hashes, session or auth tokens).
    pub fn entry(
        &self,
        action: AdminAction,
        target_user_id: Option<UserId>,
        target_email: Option<EmailAddress>,
        outcome: impl Into<String>,
        details: Option<serde_json::Value>,
    ) -> AdminAuditEntry {
        AdminAuditEntry {
            occurred_at: self.occurred_at,
            actor: self.actor.clone(),
            action,
            target_user_id,
            target_email,
            outcome: outcome.into(),
            details,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AdminAuditEntry {
    occurred_at: DateTime<Utc>,
    actor: AdminActor,
    action: AdminAction,
    target_user_id: Option<UserId>,
    target_email: Option<EmailAddress>,
    outcome: String,
    details: Option<serde_json::Value>,
}

impl AdminAuditEntry {
    pub fn occurred_at(&self) -> &DateTime<Utc> {
        &self.occurred_at
    }

    pub fn origin(&self) -> &str {
        self.actor.origin()
    }

    pub fn actor_user_id(&self) -> Option<&UserId> {
        self.actor.user()
    }

    pub fn action(&self) -> AdminAction {
        self.action
    }

    pub fn target_user_id(&self) -> Option<&UserId> {
        self.target_user_id.as_ref()
    }

    pub fn target_email(&self) -> Option<&EmailAddress> {
        self.target_email.as_ref()
    }

    pub fn outcome(&self) -> &str {
        &self.outcome
    }

    pub fn details(&self) -> Option<&serde_json::Value> {
        self.details.as_ref()
    }
}
