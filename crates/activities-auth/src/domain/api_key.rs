use std::time::Duration;

use activities_core::domain::shared::{IClock, UserId};
use base64::Engine;
use derive_more::Constructor;
use rand::RngExt;
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::domain::UserService;

#[derive(Debug, Clone, Copy)]
pub enum APIPermission {
    Read,
    Write,
    Delete,
}

#[derive(Debug, Clone, PartialEq)]
pub struct APIKeyId(String);

impl APIKeyId {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        uuid::Uuid::new_v4().into()
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for APIKeyId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl From<String> for APIKeyId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<uuid::Uuid> for APIKeyId {
    fn from(value: uuid::Uuid) -> Self {
        Self(value.to_string())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct APIKeyValue(String);

#[derive(Debug, Clone, Constructor, PartialEq)]
pub struct HashedAPIKeyValue(String);

#[derive(Debug, Clone, Constructor, PartialEq)]
pub struct MaskedAPIKeyValue(String);

#[derive(Debug, Clone, Constructor)]
pub struct APIKey {
    user: UserId,
    id: APIKeyId,
    name: String,
    permission: APIPermission,
    value: APIKeyValue,
    expire_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, Constructor)]
pub struct HashedAPIKey {
    user: UserId,
    id: APIKeyId,
    name: String,
    permission: APIPermission,
    value: HashedAPIKeyValue,
    display: MaskedAPIKeyValue,
    expire_at: Option<chrono::DateTime<chrono::Utc>>,
    disabled_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[allow(clippy::new_without_default)]
impl APIKeyValue {
    pub fn new() -> Self {
        let mut rng = rand::rng();
        let mut random_bytes = [0u8; 24];
        rng.fill(&mut random_bytes);

        Self(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(random_bytes))
    }

    pub fn as_hash(&self) -> HashedAPIKeyValue {
        let mut hasher = Sha256::new();
        hasher.update(self.0.as_bytes());
        HashedAPIKeyValue::new(const_hex::encode(hasher.finalize()))
    }

    pub fn as_display(&self) -> MaskedAPIKeyValue {
        let start = self.0[..3].to_string();
        let end = self.0[self.0.len() - 3..].to_string();
        MaskedAPIKeyValue::new(format!("{}***{}", start, end))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl HashedAPIKeyValue {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl MaskedAPIKeyValue {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for APIKeyValue {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl From<String> for APIKeyValue {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl APIKey {
    pub fn user(&self) -> &UserId {
        &self.user
    }
    pub fn id(&self) -> &APIKeyId {
        &self.id
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn permission(&self) -> &APIPermission {
        &self.permission
    }
    pub fn value(&self) -> &APIKeyValue {
        &self.value
    }
    pub fn expire_at(&self) -> &Option<chrono::DateTime<chrono::Utc>> {
        &self.expire_at
    }

    pub fn as_hash(&self) -> HashedAPIKey {
        HashedAPIKey {
            user: self.user.clone(),
            id: self.id.clone(),
            name: self.name.clone(),
            permission: self.permission,
            value: self.value.as_hash(),
            display: self.value.as_display(),
            expire_at: self.expire_at,
            disabled_at: None,
        }
    }
}

impl HashedAPIKey {
    pub fn user(&self) -> &UserId {
        &self.user
    }
    pub fn id(&self) -> &APIKeyId {
        &self.id
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn permission(&self) -> &APIPermission {
        &self.permission
    }
    pub fn value(&self) -> &HashedAPIKeyValue {
        &self.value
    }
    pub fn display(&self) -> &MaskedAPIKeyValue {
        &self.display
    }
    pub fn expire_at(&self) -> &Option<chrono::DateTime<chrono::Utc>> {
        &self.expire_at
    }
    pub fn disabled_at(&self) -> &Option<chrono::DateTime<chrono::Utc>> {
        &self.disabled_at
    }

    pub fn active(&self, now: &chrono::DateTime<chrono::Utc>) -> bool {
        if self.disabled_at.is_some() {
            return false;
        }

        if let Some(expire_at) = &self.expire_at {
            return now.lt(expire_at);
        }

        true
    }

    pub fn disable(mut self, at: chrono::DateTime<chrono::Utc>) -> Self {
        self.disabled_at.get_or_insert(at);
        self
    }
}

#[derive(Debug, Clone, Constructor, PartialEq)]
pub struct APIKeyActivity {
    user: UserId,
    key: APIKeyId,
    activity: String,
    timestamp: chrono::DateTime<chrono::Utc>,
}

impl APIKeyActivity {
    pub fn user(&self) -> &UserId {
        &self.user
    }
    pub fn key(&self) -> &APIKeyId {
        &self.key
    }
    pub fn activity(&self) -> &str {
        &self.activity
    }
    pub fn timestamp(&self) -> &chrono::DateTime<chrono::Utc> {
        &self.timestamp
    }
}

#[derive(Error, Debug)]
pub enum APIKeyError {
    #[error("Key does not exist.")]
    KeyDoesNotExist,
    #[error("Invalid user for this key.")]
    UserDoesNotOwnKey,
    #[error("Key is expired or disabled.")]
    KeyExpiredOrDisabled,
    #[error("Storage error.")]
    StorageError(anyhow::Error),
}

pub trait IAPIKeyService: Clone + Send + Sync + 'static {
    fn create_key(
        &self,
        user: &UserId,
        permission: APIPermission,
        name: String,
        duration: Option<Duration>,
    ) -> impl Future<Output = Result<APIKey, APIKeyError>> + Send;

    fn disable_key(
        &self,
        user: &UserId,
        key: &APIKeyId,
    ) -> impl Future<Output = Result<(), APIKeyError>> + Send;

    fn delete_key(
        &self,
        user: &UserId,
        key: &APIKeyId,
    ) -> impl Future<Output = Result<(), APIKeyError>> + Send;

    fn match_value_to_key(
        &self,
        value: APIKeyValue,
    ) -> impl Future<Output = Result<HashedAPIKey, APIKeyError>> + Send;

    fn log_key_activity(
        &self,
        key: &APIKeyId,
        activity: String,
        timestamp: chrono::DateTime<chrono::Utc>,
    ) -> impl Future<Output = Result<(), APIKeyError>> + Send;

    fn get_key_activity_logs(
        &self,
        user: &UserId,
        key: &APIKeyId,
    ) -> impl Future<Output = Result<Vec<APIKeyActivity>, APIKeyError>> + Send;
}

pub trait IAPIKeyRepository: Clone + Send + Sync + 'static {
    fn save_key(&self, key: HashedAPIKey) -> impl Future<Output = Result<(), APIKeyError>> + Send;
    fn delete_key(
        &self,
        user: &UserId,
        key: &APIKeyId,
    ) -> impl Future<Output = Result<(), APIKeyError>> + Send;
    fn get_key_by_id(
        &self,
        key: &APIKeyId,
    ) -> impl Future<Output = Result<HashedAPIKey, APIKeyError>> + Send;
    fn get_key_by_hash(
        &self,
        hash: &HashedAPIKeyValue,
    ) -> impl Future<Output = Result<HashedAPIKey, APIKeyError>> + Send;

    fn save_key_activity(
        &self,
        activity: &APIKeyActivity,
    ) -> impl Future<Output = Result<(), APIKeyError>> + Send;
    fn get_key_activity_logs(
        &self,
        user: &UserId,
        key: &APIKeyId,
    ) -> impl Future<Output = Result<Vec<APIKeyActivity>, APIKeyError>> + Send;
}

impl<UR, SS, MP, C> IAPIKeyService for UserService<UR, SS, MP, C>
where
    UR: IAPIKeyRepository,
    SS: Clone + Send + Sync + 'static,
    MP: Clone + Send + Sync + 'static,
    C: IClock,
{
    #[tracing::instrument(skip_all)]
    async fn create_key(
        &self,
        user: &UserId,
        permission: APIPermission,
        name: String,
        duration: Option<std::time::Duration>,
    ) -> Result<APIKey, APIKeyError> {
        let expire_at = duration
            .map(|duration| {
                chrono::Duration::from_std(duration)
                    .map(|duration| self.clock.now() + duration)
                    .map_err(|err| APIKeyError::StorageError(err.into()))
            })
            .transpose()?;

        let key = APIKey::new(
            user.clone(),
            APIKeyId::new(),
            name,
            permission,
            APIKeyValue::new(),
            expire_at,
        );

        self.user_repository.save_key(key.as_hash()).await?;

        Ok(key)
    }

    #[tracing::instrument(skip_all)]
    async fn disable_key(&self, user: &UserId, key: &APIKeyId) -> Result<(), APIKeyError> {
        let existing = self.user_repository.get_key_by_id(key).await?;

        if existing.user() != user {
            return Err(APIKeyError::UserDoesNotOwnKey);
        }

        let disabled = existing.disable(self.clock.now());

        self.user_repository.save_key(disabled).await
    }

    #[tracing::instrument(skip_all)]
    async fn delete_key(&self, user: &UserId, key: &APIKeyId) -> Result<(), APIKeyError> {
        let existing = self.user_repository.get_key_by_id(key).await?;

        if existing.user() != user {
            return Err(APIKeyError::UserDoesNotOwnKey);
        }

        self.user_repository.delete_key(user, key).await
    }

    #[tracing::instrument(skip_all)]
    async fn match_value_to_key(&self, value: APIKeyValue) -> Result<HashedAPIKey, APIKeyError> {
        let key = self
            .user_repository
            .get_key_by_hash(&value.as_hash())
            .await?;

        if !key.active(&self.clock.now()) {
            return Err(APIKeyError::KeyExpiredOrDisabled);
        }

        Ok(key)
    }

    #[tracing::instrument(skip_all)]
    async fn log_key_activity(
        &self,
        key: &APIKeyId,
        activity: String,
        timestamp: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), APIKeyError> {
        let existing = self.user_repository.get_key_by_id(key).await?;
        let activity = APIKeyActivity::new(
            existing.user().clone(),
            existing.id().clone(),
            activity,
            timestamp,
        );

        self.user_repository.save_key_activity(&activity).await
    }

    #[tracing::instrument(skip_all)]
    async fn get_key_activity_logs(
        &self,
        user: &UserId,
        key: &APIKeyId,
    ) -> Result<Vec<APIKeyActivity>, APIKeyError> {
        let existing = self.user_repository.get_key_by_id(key).await?;

        if existing.user() != user {
            return Err(APIKeyError::UserDoesNotOwnKey);
        }

        self.user_repository.get_key_activity_logs(user, key).await
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use sha2::{Digest, Sha256};

    use super::*;

    fn test_user() -> UserId {
        UserId::test_default()
    }

    fn expected_hash(value: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(value.as_bytes());
        const_hex::encode(hasher.finalize())
    }

    fn sample_api_key(expire_at: Option<chrono::DateTime<Utc>>) -> APIKey {
        APIKey::new(
            test_user(),
            APIKeyId::from("key-1"),
            "key-name".to_string(),
            APIPermission::Read,
            APIKeyValue::from("abcdefghijklmnop"),
            expire_at,
        )
    }

    fn sample_hashed_api_key(
        expire_at: Option<chrono::DateTime<Utc>>,
        disabled_at: Option<chrono::DateTime<Utc>>,
    ) -> HashedAPIKey {
        sample_hashed_api_key_for_user_and_expiry(test_user(), expire_at, disabled_at)
    }

    fn sample_hashed_api_key_for_user(user: UserId) -> HashedAPIKey {
        sample_hashed_api_key_for_user_and_expiry(user, None, None)
    }

    fn sample_hashed_api_key_for_user_and_expiry(
        user: UserId,
        expire_at: Option<chrono::DateTime<Utc>>,
        disabled_at: Option<chrono::DateTime<Utc>>,
    ) -> HashedAPIKey {
        HashedAPIKey::new(
            user,
            APIKeyId::from("key-1"),
            "key-name".to_string(),
            APIPermission::Read,
            HashedAPIKeyValue::new("hash".to_string()),
            MaskedAPIKeyValue::new("abc***nop".to_string()),
            expire_at,
            disabled_at,
        )
    }

    fn reference_time() -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2030, 1, 1, 0, 0, 0).unwrap()
    }

    #[test]
    fn test_new_value_is_url_safe_base64_without_padding() {
        let value = APIKeyValue::new();

        assert_eq!(value.0.len(), 32);
        assert!(
            !value.0.contains('=') && !value.0.contains('+') && !value.0.contains('/'),
            "unexpected characters in generated value: {}",
            value.0
        );
        let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(&value.0)
            .expect("generated value should be valid base64");
        assert_eq!(decoded.len(), 24);
    }

    #[test]
    fn test_new_value_is_random() {
        let first = APIKeyValue::new();
        let second = APIKeyValue::new();

        assert_ne!(first, second);
    }

    #[test]
    fn test_new_id_is_a_uuid() {
        let id = APIKeyId::new();

        assert!(
            uuid::Uuid::parse_str(&id.0).is_ok(),
            "generated id is not a UUID: {}",
            id.0
        );
    }

    #[test]
    fn test_new_id_is_random() {
        let first = APIKeyId::new();
        let second = APIKeyId::new();

        assert_ne!(first, second);
    }

    #[test]
    fn test_id_from_str_and_string_are_equal() {
        let from_str = APIKeyId::from("key-1");
        let from_string = APIKeyId::from(String::from("key-1"));

        assert_eq!(from_str, from_string);
    }

    #[test]
    fn test_id_from_uuid_uses_hyphenated_representation() {
        let uuid = uuid::Uuid::new_v4();

        assert_eq!(APIKeyId::from(uuid), APIKeyId::from(uuid.to_string()));
    }

    #[test]
    fn test_as_hash_matches_sha256_hex_of_value() {
        let value = APIKeyValue::from("some-api-key-value");

        let hashed = value.as_hash();

        assert_eq!(hashed, HashedAPIKeyValue::new(expected_hash(&value.0)));
        assert_eq!(hashed.0.len(), 64);
    }

    #[test]
    fn test_as_hash_is_deterministic() {
        let value = APIKeyValue::from("repeatable-value");

        assert_eq!(value.as_hash(), value.as_hash());
    }

    #[test]
    fn test_as_hash_differs_for_different_values() {
        let first = APIKeyValue::from("first-value").as_hash();
        let second = APIKeyValue::from("second-value").as_hash();

        assert_ne!(first, second);
    }

    #[test]
    fn test_as_display_masks_middle_of_value() {
        let value = APIKeyValue::from("abcdefghijklmnop");

        assert_eq!(value.as_display().0, "abc***nop");
    }

    #[test]
    fn test_value_from_str_and_string_are_equal() {
        let from_str = APIKeyValue::from("same-value");
        let from_string = APIKeyValue::from(String::from("same-value"));

        assert_eq!(from_str, from_string);
        assert_eq!(from_str.0, "same-value");
    }

    #[test]
    fn test_api_key_accessors() {
        let expire_at = Utc.with_ymd_and_hms(2030, 1, 2, 3, 4, 5).unwrap();
        let key = sample_api_key(Some(expire_at));

        assert_eq!(key.user(), &test_user());
        assert_eq!(key.id(), &APIKeyId::from("key-1"));
        assert!(matches!(key.permission(), APIPermission::Read));
        assert_eq!(key.value(), &APIKeyValue::from("abcdefghijklmnop"));
        assert_eq!(key.expire_at(), &Some(expire_at));
    }

    #[test]
    fn test_api_key_without_expiry() {
        let key = sample_api_key(None);

        assert!(key.expire_at().is_none());
    }

    #[test]
    fn test_api_key_as_hash_preserves_metadata_and_hashes_value() {
        let expire_at = Utc.with_ymd_and_hms(2030, 1, 2, 3, 4, 5).unwrap();
        let key = sample_api_key(Some(expire_at));

        let hashed = key.as_hash();

        assert_eq!(hashed.user(), key.user());
        assert_eq!(hashed.id(), key.id());
        assert!(matches!(hashed.permission(), APIPermission::Read));
        assert_eq!(hashed.value(), &key.value().as_hash());
        assert_eq!(hashed.display(), &key.value().as_display());
        assert_eq!(hashed.expire_at(), key.expire_at());
    }

    #[test]
    fn test_api_key_as_hash_is_born_enabled() {
        let hashed = sample_api_key(None).as_hash();

        assert!(hashed.disabled_at().is_none());
        assert!(hashed.active(&reference_time()));
    }

    #[test]
    fn test_hashed_api_key_does_not_expose_raw_value() {
        let key = sample_api_key(None);

        let hashed = key.as_hash();

        assert_ne!(hashed.value().0, key.value().0);
        assert_eq!(hashed.display().0, "abc***nop");
    }

    #[test]
    fn test_hashed_api_key_accessors() {
        let expire_at = Utc.with_ymd_and_hms(2030, 1, 2, 3, 4, 5).unwrap();
        let hashed = sample_hashed_api_key(Some(expire_at), None);

        assert_eq!(hashed.user(), &test_user());
        assert_eq!(hashed.id(), &APIKeyId::from("key-1"));
        assert!(matches!(hashed.permission(), APIPermission::Read));
        assert_eq!(hashed.value(), &HashedAPIKeyValue::new("hash".to_string()));
        assert_eq!(
            hashed.display(),
            &MaskedAPIKeyValue::new("abc***nop".to_string())
        );
        assert_eq!(hashed.expire_at(), &Some(expire_at));
        assert_eq!(hashed.disabled_at(), &None);
    }

    #[test]
    fn test_disabled_sets_timestamp_and_makes_key_inactive() {
        let disabled_at = reference_time();
        let hashed = sample_hashed_api_key(None, None).disable(disabled_at);

        assert_eq!(hashed.disabled_at(), &Some(disabled_at));
        assert!(!hashed.active(&reference_time()));
    }

    #[test]
    fn test_disabled_overrides_future_expiry() {
        let expire_at = reference_time() + chrono::Duration::days(30);
        let hashed = sample_hashed_api_key(Some(expire_at), None).disable(reference_time());

        assert!(!hashed.active(&reference_time()));
    }

    #[test]
    fn test_disabled_twice_keeps_first_timestamp() {
        let first = reference_time();
        let second = reference_time() + chrono::Duration::days(1);
        let hashed = sample_hashed_api_key(None, None)
            .disable(first)
            .disable(second);

        assert_eq!(hashed.disabled_at(), &Some(first));
    }

    #[test]
    fn test_disabled_preserves_other_fields() {
        let expire_at = reference_time() + chrono::Duration::days(7);
        let original = sample_hashed_api_key(Some(expire_at), None);
        let expected = original.clone();

        let hashed = original.disable(reference_time());

        assert_eq!(hashed.user(), expected.user());
        assert_eq!(hashed.id(), expected.id());
        assert!(matches!(hashed.permission(), APIPermission::Read));
        assert_eq!(hashed.value(), expected.value());
        assert_eq!(hashed.display(), expected.display());
        assert_eq!(hashed.expire_at(), expected.expire_at());
    }

    #[test]
    fn test_hashed_api_key_active_without_expiry_or_disabled_at() {
        let hashed = sample_hashed_api_key(None, None);

        assert!(hashed.active(&reference_time()));
    }

    #[test]
    fn test_hashed_api_key_active_before_expiry() {
        let expire_at = reference_time() + chrono::Duration::days(1);
        let hashed = sample_hashed_api_key(Some(expire_at), None);

        assert!(hashed.active(&reference_time()));
    }

    #[test]
    fn test_hashed_api_key_inactive_at_expiry() {
        let hashed = sample_hashed_api_key(Some(reference_time()), None);

        assert!(!hashed.active(&reference_time()));
    }

    #[test]
    fn test_hashed_api_key_inactive_after_expiry() {
        let expire_at = reference_time() - chrono::Duration::days(1);
        let hashed = sample_hashed_api_key(Some(expire_at), None);

        assert!(!hashed.active(&reference_time()));
    }

    #[test]
    fn test_hashed_api_key_inactive_when_disabled() {
        let hashed = sample_hashed_api_key(None, Some(reference_time()));

        assert!(!hashed.active(&reference_time()));
    }

    #[test]
    fn test_hashed_api_key_active_cases() {
        let cases = [
            (None, None, true),
            (
                Some(reference_time() + chrono::Duration::days(1)),
                None,
                true,
            ),
            (Some(reference_time()), None, false),
            (
                Some(reference_time() - chrono::Duration::days(1)),
                None,
                false,
            ),
            (None, Some(reference_time()), false),
            (
                Some(reference_time() + chrono::Duration::days(1)),
                Some(reference_time() - chrono::Duration::days(1)),
                false,
            ),
        ];

        for (expire_at, disabled_at, expected) in cases {
            let hashed = sample_hashed_api_key(expire_at, disabled_at);

            assert_eq!(
                hashed.active(&reference_time()),
                expected,
                "unexpected active() for expire_at={expire_at:?}, disabled_at={disabled_at:?}"
            );
        }
    }

    mod service {
        use std::sync::{Arc, Mutex};

        use anyhow::anyhow;

        use activities_core::domain::clock::FakeClock;

        use crate::domain::{
            AuthMethods,
            test_utils::{MockMailProvider, MockSessionService},
        };

        use super::*;

        mockall::mock! {
            pub APIKeyRepository {}

            impl Clone for APIKeyRepository {
                fn clone(&self) -> Self;
            }

            impl IAPIKeyRepository for APIKeyRepository {
                async fn save_key(&self, key: HashedAPIKey) -> Result<(), APIKeyError>;
                async fn delete_key(
                    &self,
                    user: &UserId,
                    key: &APIKeyId,
                ) -> Result<(), APIKeyError>;
                async fn get_key_by_id(
                    &self,
                    key: &APIKeyId,
                ) -> Result<HashedAPIKey, APIKeyError>;
                async fn get_key_by_hash(
                    &self,
                    hash: &HashedAPIKeyValue,
                ) -> Result<HashedAPIKey, APIKeyError>;
                async fn save_key_activity(
                    &self,
                    activity: &APIKeyActivity,
                ) -> Result<(), APIKeyError>;
                async fn get_key_activity_logs(
                    &self,
                    user: &UserId,
                    key: &APIKeyId,
                ) -> Result<Vec<APIKeyActivity>, APIKeyError>;
            }
        }

        type TestService =
            UserService<MockAPIKeyRepository, MockSessionService, MockMailProvider, FakeClock>;

        fn service(repository: MockAPIKeyRepository) -> TestService {
            service_at(repository, reference_time())
        }

        fn service_at(repository: MockAPIKeyRepository, now: chrono::DateTime<Utc>) -> TestService {
            UserService::new(
                Arc::new(repository),
                MockSessionService::new(),
                AuthMethods::new(false),
                Arc::new(MockMailProvider::new()),
                FakeClock::new(now),
            )
        }

        fn key_id(value: &str) -> APIKeyId {
            APIKeyId::from(value)
        }

        fn hashed_key_for(
            value: &APIKeyValue,
            expire_at: Option<chrono::DateTime<Utc>>,
            disabled_at: Option<chrono::DateTime<Utc>>,
        ) -> HashedAPIKey {
            HashedAPIKey::new(
                test_user(),
                key_id("key-1"),
                "key-name".to_string(),
                APIPermission::Read,
                value.as_hash(),
                value.as_display(),
                expire_at,
                disabled_at,
            )
        }

        #[tokio::test]
        async fn test_create_key_stores_hashed_key_and_returns_raw_value() {
            let saved = Arc::new(Mutex::new(None));
            let saved_for_mock = Arc::clone(&saved);

            let mut repository = MockAPIKeyRepository::new();
            repository.expect_save_key().times(1).returning(move |key| {
                *saved_for_mock.lock().unwrap() = Some(key);
                Ok(())
            });

            let key = service(repository)
                .create_key(
                    &test_user(),
                    APIPermission::Write,
                    "key-name".to_string(),
                    Some(Duration::from_secs(3600)),
                )
                .await
                .expect("creating a key should succeed");

            assert_eq!(key.user(), &test_user());
            assert_eq!(key.name(), "key-name");
            assert!(matches!(key.permission(), APIPermission::Write));
            assert_eq!(key.value().0.len(), 32);

            assert_eq!(
                key.expire_at(),
                &Some(reference_time() + chrono::Duration::hours(1))
            );

            let saved = saved
                .lock()
                .unwrap()
                .take()
                .expect("the hashed key should have been stored");
            assert_eq!(saved.user(), key.user());
            assert_eq!(saved.id(), key.id());
            assert_eq!(saved.name(), key.name());
            assert!(matches!(saved.permission(), APIPermission::Write));
            assert_eq!(saved.value(), &key.value().as_hash());
            assert_eq!(saved.display(), &key.value().as_display());
            assert_eq!(saved.expire_at(), key.expire_at());
            assert!(saved.disabled_at().is_none());
        }

        #[tokio::test]
        async fn test_create_key_without_duration_never_expires() {
            let mut repository = MockAPIKeyRepository::new();
            repository
                .expect_save_key()
                .times(1)
                .withf(|key| key.expire_at().is_none())
                .returning(|_| Ok(()));

            let key = service(repository)
                .create_key(
                    &test_user(),
                    APIPermission::Read,
                    "forever".to_string(),
                    None,
                )
                .await
                .expect("creating a key should succeed");

            assert!(key.expire_at().is_none());
        }

        #[tokio::test]
        async fn test_create_key_generates_distinct_ids_and_values() {
            let mut repository = MockAPIKeyRepository::new();
            repository.expect_save_key().times(2).returning(|_| Ok(()));

            let service = service(repository);
            let first = service
                .create_key(&test_user(), APIPermission::Read, "first".to_string(), None)
                .await
                .unwrap();
            let second = service
                .create_key(
                    &test_user(),
                    APIPermission::Read,
                    "second".to_string(),
                    None,
                )
                .await
                .unwrap();

            assert_ne!(first.id(), second.id());
            assert_ne!(first.value(), second.value());
        }

        #[tokio::test]
        async fn test_create_key_storage_error_is_returned() {
            let mut repository = MockAPIKeyRepository::new();
            repository
                .expect_save_key()
                .returning(|_| Err(APIKeyError::StorageError(anyhow!("storage failure"))));

            let result = service(repository)
                .create_key(&test_user(), APIPermission::Read, "name".to_string(), None)
                .await;

            assert!(matches!(result, Err(APIKeyError::StorageError(_))));
        }

        #[tokio::test]
        async fn test_create_key_rejects_unrepresentable_duration() {
            let mut repository = MockAPIKeyRepository::new();
            repository.expect_save_key().times(0);

            let result = service(repository)
                .create_key(
                    &test_user(),
                    APIPermission::Read,
                    "name".to_string(),
                    Some(Duration::MAX),
                )
                .await;

            assert!(matches!(result, Err(APIKeyError::StorageError(_))));
        }

        #[tokio::test]
        async fn test_disable_key_marks_key_disabled() {
            let id = key_id("key-1");
            let id_for_get = id.clone();
            let id_for_save = id.clone();

            let mut repository = MockAPIKeyRepository::new();
            repository
                .expect_get_key_by_id()
                .times(1)
                .withf(move |key| key == &id_for_get)
                .returning(|_| Ok(sample_hashed_api_key(None, None)));
            repository
                .expect_save_key()
                .times(1)
                .withf(move |key| {
                    key.id() == &id_for_save
                        && key.disabled_at() == &Some(reference_time())
                        && key.value() == &HashedAPIKeyValue::new("hash".to_string())
                })
                .returning(|_| Ok(()));

            service(repository)
                .disable_key(&test_user(), &id)
                .await
                .expect("disabling an owned key should succeed");
        }

        #[tokio::test]
        async fn test_disable_key_keeps_original_disabled_at() {
            let original_disabled_at = reference_time() - chrono::Duration::days(1);

            let mut repository = MockAPIKeyRepository::new();
            repository
                .expect_get_key_by_id()
                .returning(move |_| Ok(sample_hashed_api_key(None, Some(original_disabled_at))));
            repository
                .expect_save_key()
                .times(1)
                .withf(move |key| key.disabled_at() == &Some(original_disabled_at))
                .returning(|_| Ok(()));

            service(repository)
                .disable_key(&test_user(), &key_id("key-1"))
                .await
                .expect("disabling an already-disabled owned key should succeed");
        }

        #[tokio::test]
        async fn test_disable_key_rejects_other_users_key() {
            let mut repository = MockAPIKeyRepository::new();
            repository
                .expect_get_key_by_id()
                .returning(|_| Ok(sample_hashed_api_key_for_user(UserId::from("other"))));
            repository.expect_save_key().times(0);

            let result = service(repository)
                .disable_key(&test_user(), &key_id("key-1"))
                .await;

            assert!(matches!(result, Err(APIKeyError::UserDoesNotOwnKey)));
        }

        #[tokio::test]
        async fn test_disable_key_unknown_key_is_rejected() {
            let mut repository = MockAPIKeyRepository::new();
            repository
                .expect_get_key_by_id()
                .returning(|_| Err(APIKeyError::KeyDoesNotExist));
            repository.expect_save_key().times(0);

            let result = service(repository)
                .disable_key(&test_user(), &key_id("unknown"))
                .await;

            assert!(matches!(result, Err(APIKeyError::KeyDoesNotExist)));
        }

        #[tokio::test]
        async fn test_disable_key_storage_error_is_returned() {
            let mut repository = MockAPIKeyRepository::new();
            repository
                .expect_get_key_by_id()
                .returning(|_| Ok(sample_hashed_api_key(None, None)));
            repository
                .expect_save_key()
                .returning(|_| Err(APIKeyError::StorageError(anyhow!("storage failure"))));

            let result = service(repository)
                .disable_key(&test_user(), &key_id("key-1"))
                .await;

            assert!(matches!(result, Err(APIKeyError::StorageError(_))));
        }

        #[tokio::test]
        async fn test_delete_key_deletes_owned_key() {
            let id = key_id("key-1");
            let id_for_get = id.clone();
            let id_for_delete = id.clone();

            let mut repository = MockAPIKeyRepository::new();
            repository
                .expect_get_key_by_id()
                .times(1)
                .withf(move |key| key == &id_for_get)
                .returning(|_| Ok(sample_hashed_api_key(None, None)));
            repository
                .expect_delete_key()
                .times(1)
                .withf(move |user, key| user == &test_user() && key == &id_for_delete)
                .returning(|_, _| Ok(()));

            service(repository)
                .delete_key(&test_user(), &id)
                .await
                .expect("deleting an owned key should succeed");
        }

        #[tokio::test]
        async fn test_delete_key_rejects_other_users_key() {
            let mut repository = MockAPIKeyRepository::new();
            repository
                .expect_get_key_by_id()
                .returning(|_| Ok(sample_hashed_api_key_for_user(UserId::from("other"))));
            repository.expect_delete_key().times(0);

            let result = service(repository)
                .delete_key(&test_user(), &key_id("key-1"))
                .await;

            assert!(matches!(result, Err(APIKeyError::UserDoesNotOwnKey)));
        }

        #[tokio::test]
        async fn test_delete_key_unknown_key_is_rejected() {
            let mut repository = MockAPIKeyRepository::new();
            repository
                .expect_get_key_by_id()
                .returning(|_| Err(APIKeyError::KeyDoesNotExist));
            repository.expect_delete_key().times(0);

            let result = service(repository)
                .delete_key(&test_user(), &key_id("unknown"))
                .await;

            assert!(matches!(result, Err(APIKeyError::KeyDoesNotExist)));
        }

        #[tokio::test]
        async fn test_delete_key_storage_error_is_returned() {
            let mut repository = MockAPIKeyRepository::new();
            repository
                .expect_get_key_by_id()
                .returning(|_| Ok(sample_hashed_api_key(None, None)));
            repository
                .expect_delete_key()
                .returning(|_, _| Err(APIKeyError::StorageError(anyhow!("storage failure"))));

            let result = service(repository)
                .delete_key(&test_user(), &key_id("key-1"))
                .await;

            assert!(matches!(result, Err(APIKeyError::StorageError(_))));
        }

        #[tokio::test]
        async fn test_match_value_to_key_looks_up_hash_and_returns_active_key() {
            let value = APIKeyValue::from("abcdefghijklmnop");
            let expected_hash = value.as_hash();
            let expected_for_mock = expected_hash.clone();

            let mut repository = MockAPIKeyRepository::new();
            repository
                .expect_get_key_by_hash()
                .times(1)
                .withf(move |hash| hash == &expected_for_mock)
                .returning(move |_| {
                    Ok(hashed_key_for(
                        &value,
                        Some(reference_time() + chrono::Duration::days(1)),
                        None,
                    ))
                });

            let matched = service(repository)
                .match_value_to_key(APIKeyValue::from("abcdefghijklmnop"))
                .await
                .expect("an active key should match");

            assert_eq!(matched.value(), &expected_hash);
            assert_eq!(matched.id(), &key_id("key-1"));
        }

        #[tokio::test]
        async fn test_match_value_to_key_rejects_expired_key() {
            let value = APIKeyValue::from("abcdefghijklmnop");
            let mut repository = MockAPIKeyRepository::new();
            repository.expect_get_key_by_hash().returning(move |_| {
                Ok(hashed_key_for(
                    &APIKeyValue::from("abcdefghijklmnop"),
                    Some(reference_time() - chrono::Duration::days(1)),
                    None,
                ))
            });

            let result = service(repository).match_value_to_key(value).await;

            assert!(matches!(result, Err(APIKeyError::KeyExpiredOrDisabled)));
        }

        #[tokio::test]
        async fn test_match_value_to_key_rejects_key_expiring_at_now() {
            let value = APIKeyValue::from("abcdefghijklmnop");
            let mut repository = MockAPIKeyRepository::new();
            repository.expect_get_key_by_hash().returning(move |_| {
                Ok(hashed_key_for(
                    &APIKeyValue::from("abcdefghijklmnop"),
                    Some(reference_time()),
                    None,
                ))
            });

            let result = service(repository).match_value_to_key(value).await;

            assert!(matches!(result, Err(APIKeyError::KeyExpiredOrDisabled)));
        }

        #[tokio::test]
        async fn test_match_value_to_key_rejects_disabled_key() {
            let value = APIKeyValue::from("abcdefghijklmnop");
            let mut repository = MockAPIKeyRepository::new();
            repository.expect_get_key_by_hash().returning(move |_| {
                Ok(hashed_key_for(
                    &APIKeyValue::from("abcdefghijklmnop"),
                    None,
                    Some(reference_time() - chrono::Duration::days(1)),
                ))
            });

            let result = service(repository).match_value_to_key(value).await;

            assert!(matches!(result, Err(APIKeyError::KeyExpiredOrDisabled)));
        }

        #[tokio::test]
        async fn test_match_value_to_key_unknown_key_is_rejected() {
            let mut repository = MockAPIKeyRepository::new();
            repository
                .expect_get_key_by_hash()
                .returning(|_| Err(APIKeyError::KeyDoesNotExist));

            let result = service(repository)
                .match_value_to_key(APIKeyValue::from("abcdefghijklmnop"))
                .await;

            assert!(matches!(result, Err(APIKeyError::KeyDoesNotExist)));
        }

        #[tokio::test]
        async fn test_match_value_to_key_storage_error_is_returned() {
            let mut repository = MockAPIKeyRepository::new();
            repository
                .expect_get_key_by_hash()
                .returning(|_| Err(APIKeyError::StorageError(anyhow!("storage failure"))));

            let result = service(repository)
                .match_value_to_key(APIKeyValue::from("abcdefghijklmnop"))
                .await;

            assert!(matches!(result, Err(APIKeyError::StorageError(_))));
        }

        #[tokio::test]
        async fn test_log_key_activity_uses_keys_owner() {
            let id = key_id("key-1");
            let timestamp = reference_time();
            let id_for_activity = id.clone();

            let mut repository = MockAPIKeyRepository::new();
            repository
                .expect_get_key_by_id()
                .times(1)
                .withf(move |key| key == &id)
                .returning(|_| Ok(sample_hashed_api_key(None, None)));
            repository
                .expect_save_key_activity()
                .times(1)
                .withf(move |activity| {
                    activity.user() == &test_user()
                        && activity.key() == &id_for_activity
                        && activity.activity() == "read activities"
                        && activity.timestamp() == &timestamp
                })
                .returning(|_| Ok(()));

            service(repository)
                .log_key_activity(&key_id("key-1"), "read activities".to_string(), timestamp)
                .await
                .expect("logging activity for a known key should succeed");
        }

        #[tokio::test]
        async fn test_log_key_activity_unknown_key_is_rejected() {
            let mut repository = MockAPIKeyRepository::new();
            repository
                .expect_get_key_by_id()
                .returning(|_| Err(APIKeyError::KeyDoesNotExist));
            repository.expect_save_key_activity().times(0);

            let result = service(repository)
                .log_key_activity(&key_id("unknown"), "activity".to_string(), reference_time())
                .await;

            assert!(matches!(result, Err(APIKeyError::KeyDoesNotExist)));
        }

        #[tokio::test]
        async fn test_log_key_activity_storage_error_is_returned() {
            let mut repository = MockAPIKeyRepository::new();
            repository
                .expect_get_key_by_id()
                .returning(|_| Ok(sample_hashed_api_key(None, None)));
            repository
                .expect_save_key_activity()
                .returning(|_| Err(APIKeyError::StorageError(anyhow!("storage failure"))));

            let result = service(repository)
                .log_key_activity(&key_id("key-1"), "activity".to_string(), reference_time())
                .await;

            assert!(matches!(result, Err(APIKeyError::StorageError(_))));
        }

        #[tokio::test]
        async fn test_get_key_activity_logs_returns_repository_logs() {
            let id = key_id("key-1");
            let expected = vec![APIKeyActivity::new(
                test_user(),
                id.clone(),
                "logged".to_string(),
                reference_time(),
            )];
            let expected_for_mock = expected.clone();
            let id_for_get = id.clone();
            let id_for_logs = id.clone();

            let mut repository = MockAPIKeyRepository::new();
            repository
                .expect_get_key_by_id()
                .times(1)
                .withf(move |key| key == &id_for_get)
                .returning(|_| Ok(sample_hashed_api_key(None, None)));
            repository
                .expect_get_key_activity_logs()
                .times(1)
                .withf(move |user, key| user == &test_user() && key == &id_for_logs)
                .returning(move |_, _| Ok(expected_for_mock.clone()));

            let logs = service(repository)
                .get_key_activity_logs(&test_user(), &id)
                .await
                .expect("fetching logs for an owned key should succeed");

            assert_eq!(logs, expected);
        }

        #[tokio::test]
        async fn test_get_key_activity_logs_rejects_other_users_key() {
            let mut repository = MockAPIKeyRepository::new();
            repository
                .expect_get_key_by_id()
                .returning(|_| Ok(sample_hashed_api_key_for_user(UserId::from("other"))));
            repository.expect_get_key_activity_logs().times(0);

            let result = service(repository)
                .get_key_activity_logs(&test_user(), &key_id("key-1"))
                .await;

            assert!(matches!(result, Err(APIKeyError::UserDoesNotOwnKey)));
        }

        #[tokio::test]
        async fn test_get_key_activity_logs_unknown_key_is_rejected() {
            let mut repository = MockAPIKeyRepository::new();
            repository
                .expect_get_key_by_id()
                .returning(|_| Err(APIKeyError::KeyDoesNotExist));
            repository.expect_get_key_activity_logs().times(0);

            let result = service(repository)
                .get_key_activity_logs(&test_user(), &key_id("unknown"))
                .await;

            assert!(matches!(result, Err(APIKeyError::KeyDoesNotExist)));
        }

        #[tokio::test]
        async fn test_get_key_activity_logs_storage_error_is_returned() {
            let mut repository = MockAPIKeyRepository::new();
            repository
                .expect_get_key_by_id()
                .returning(|_| Ok(sample_hashed_api_key(None, None)));
            repository
                .expect_get_key_activity_logs()
                .returning(|_, _| Err(APIKeyError::StorageError(anyhow!("storage failure"))));

            let result = service(repository)
                .get_key_activity_logs(&test_user(), &key_id("key-1"))
                .await;

            assert!(matches!(result, Err(APIKeyError::StorageError(_))));
        }
    }
}
