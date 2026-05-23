use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

const SERVICE_NAME: &str = "wechat-article-exporter-desktop";

pub type SecretStoreResult<T> = Result<T, SecretStoreError>;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SecretSlot {
    OfficialAccountLogin,
    ArticleReadingCredential,
}

impl SecretSlot {
    pub const ALL_CREDENTIALS: [Self; 2] =
        [Self::OfficialAccountLogin, Self::ArticleReadingCredential];

    fn key(self) -> &'static str {
        match self {
            Self::OfficialAccountLogin => "official-account-login",
            Self::ArticleReadingCredential => "article-reading-credential",
        }
    }
}

#[derive(Debug)]
pub enum SecretStoreError {
    Backend(String),
    LockPoisoned,
    UnsupportedPlatform,
}

impl fmt::Display for SecretStoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Backend(error) => write!(formatter, "secret store backend error: {error}"),
            Self::LockPoisoned => write!(formatter, "secret store lock was poisoned"),
            Self::UnsupportedPlatform => {
                write!(
                    formatter,
                    "secret store backend is not configured for this platform"
                )
            }
        }
    }
}

impl std::error::Error for SecretStoreError {}

pub trait SecretBackend {
    fn save(&self, slot: SecretSlot, value: &str) -> SecretStoreResult<()>;
    fn read(&self, slot: SecretSlot) -> SecretStoreResult<Option<String>>;
    fn delete(&self, slot: SecretSlot) -> SecretStoreResult<()>;
}

pub struct SecretStore<B> {
    backend: B,
}

impl<B> SecretStore<B>
where
    B: SecretBackend,
{
    pub fn new(backend: B) -> Self {
        Self { backend }
    }

    pub fn save(&self, slot: SecretSlot, value: &str) -> SecretStoreResult<()> {
        self.backend.save(slot, value)
    }

    pub fn read(&self, slot: SecretSlot) -> SecretStoreResult<Option<String>> {
        self.backend.read(slot)
    }

    pub fn delete(&self, slot: SecretSlot) -> SecretStoreResult<()> {
        self.backend.delete(slot)
    }

    pub fn clear_credentials(&self) -> SecretStoreResult<()> {
        for slot in SecretSlot::ALL_CREDENTIALS {
            self.delete(slot)?;
        }

        Ok(())
    }
}

#[derive(Clone, Default)]
pub struct MemorySecretBackend {
    values: Arc<Mutex<HashMap<SecretSlot, String>>>,
}

impl SecretBackend for MemorySecretBackend {
    fn save(&self, slot: SecretSlot, value: &str) -> SecretStoreResult<()> {
        self.values
            .lock()
            .map_err(|_| SecretStoreError::LockPoisoned)?
            .insert(slot, value.to_string());

        Ok(())
    }

    fn read(&self, slot: SecretSlot) -> SecretStoreResult<Option<String>> {
        Ok(self
            .values
            .lock()
            .map_err(|_| SecretStoreError::LockPoisoned)?
            .get(&slot)
            .cloned())
    }

    fn delete(&self, slot: SecretSlot) -> SecretStoreResult<()> {
        self.values
            .lock()
            .map_err(|_| SecretStoreError::LockPoisoned)?
            .remove(&slot);

        Ok(())
    }
}

#[cfg(windows)]
#[derive(Clone, Default)]
pub struct KeyringSecretBackend;

#[cfg(windows)]
impl SecretBackend for KeyringSecretBackend {
    fn save(&self, slot: SecretSlot, value: &str) -> SecretStoreResult<()> {
        keyring_entry(slot)?
            .set_password(value)
            .map_err(keyring_error)?;

        Ok(())
    }

    fn read(&self, slot: SecretSlot) -> SecretStoreResult<Option<String>> {
        match keyring_entry(slot)?.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(keyring_error(error)),
        }
    }

    fn delete(&self, slot: SecretSlot) -> SecretStoreResult<()> {
        match keyring_entry(slot)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(keyring_error(error)),
        }
    }
}

#[cfg(windows)]
pub fn production_secret_store() -> SecretStore<KeyringSecretBackend> {
    SecretStore::new(KeyringSecretBackend)
}

#[cfg(not(windows))]
pub fn production_secret_store() -> SecretStore<UnsupportedSecretBackend> {
    SecretStore::new(UnsupportedSecretBackend)
}

#[cfg(not(windows))]
#[derive(Clone, Default)]
pub struct UnsupportedSecretBackend;

#[cfg(not(windows))]
impl SecretBackend for UnsupportedSecretBackend {
    fn save(&self, _slot: SecretSlot, _value: &str) -> SecretStoreResult<()> {
        Err(SecretStoreError::UnsupportedPlatform)
    }

    fn read(&self, _slot: SecretSlot) -> SecretStoreResult<Option<String>> {
        Err(SecretStoreError::UnsupportedPlatform)
    }

    fn delete(&self, _slot: SecretSlot) -> SecretStoreResult<()> {
        Err(SecretStoreError::UnsupportedPlatform)
    }
}

#[cfg(windows)]
fn keyring_entry(slot: SecretSlot) -> SecretStoreResult<keyring::Entry> {
    keyring::Entry::new(SERVICE_NAME, slot.key()).map_err(keyring_error)
}

#[cfg(windows)]
fn keyring_error(error: keyring::Error) -> SecretStoreError {
    SecretStoreError::Backend(error.to_string())
}
