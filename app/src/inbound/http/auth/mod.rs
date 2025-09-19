use std::{marker::PhantomData, sync::Arc};

use axum::{
    extract::{FromRef, FromRequestParts},
    http::{StatusCode, request::Parts},
};
use derive_more::Constructor;

use crate::{
    domain::{
        models::UserId,
        ports::{IActivityService, ITrainingMetricService},
    },
    inbound::{http::AppState, parser::ParseFile},
};

#[derive(Debug, Clone, Constructor)]
pub struct AuthenticatedUser(UserId);

impl AuthenticatedUser {
    pub fn user(&self) -> &UserId {
        &self.0
    }
}

/// Dummy extractor that always returns the default [UserId], regardless of the request.
pub struct DefaultUserExtractor;

impl<S> FromRequestParts<S> for DefaultUserExtractor
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let user = AuthenticatedUser::new(UserId::default());

        parts.extensions.insert(user);

        Ok(Self)
    }
}

pub trait ISessionRepository: Clone + Send + Sync + 'static {
    fn dummy(&self) -> impl Future<Output = Result<(), ()>> + Send;
}

#[derive(Debug, Clone, Constructor)]
pub struct SessionRepository;
impl ISessionRepository for SessionRepository {
    async fn dummy(&self) -> Result<(), ()> {
        tracing::info!("call from dummy repo");
        Ok(())
    }
}

pub struct SessionsRepositoryWrapper<SR: ISessionRepository>(Arc<SR>);

impl<AS, PF, TMS, SR> FromRef<AppState<AS, PF, TMS, SR>> for SessionsRepositoryWrapper<SR>
where
    AS: IActivityService,
    PF: ParseFile,
    TMS: ITrainingMetricService,
    SR: ISessionRepository,
{
    fn from_ref(input: &AppState<AS, PF, TMS, SR>) -> Self {
        Self(input.session_repository.clone())
    }
}

/// Extractor that tries to extract user information from the request's session cookie.
pub struct CookieUserExtractor<SR>(PhantomData<SR>);

impl<S, SR> FromRequestParts<S> for CookieUserExtractor<SR>
where
    S: Send + Sync,
    SR: ISessionRepository,
    SessionsRepositoryWrapper<SR>: FromRef<S>,
{
    type Rejection = StatusCode;

    async fn from_request_parts(_parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let repository = SessionsRepositoryWrapper::from_ref(state);
        let _ = repository.0.dummy().await;
        Ok(Self(PhantomData))
    }
}

#[cfg(test)]
pub mod test_utils {
    use mockall::mock;

    use super::*;

    mock! {
        pub SessionRepository {}

        impl Clone for SessionRepository {
            fn clone(&self) -> Self;
        }

        impl ISessionRepository for SessionRepository {
            async fn dummy(&self) -> Result<(), ()>;
        }
    }
}
