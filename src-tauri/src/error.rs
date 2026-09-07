use serde::Serialize;

/// Application-wide error type. Serializes to a tagged object so the frontend
/// can distinguish rate-limit / auth / network failures from generic ones.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("not authenticated with {0}")]
    NotAuthenticated(String),

    #[error("rate limited by {service}; retry in {retry_after_secs}s")]
    RateLimited {
        service: String,
        retry_after_secs: u64,
    },

    #[error("network error: {0}")]
    Network(String),

    #[error("the {service} API returned an error: {message}")]
    Api { service: String, message: String },

    #[error("database error: {0}")]
    Db(String),

    #[error("keychain error: {0}")]
    Keychain(String),

    #[error("{0}")]
    Other(String),
}

impl AppError {
    pub fn other(msg: impl Into<String>) -> Self {
        AppError::Other(msg.into())
    }
}

pub type AppResult<T> = Result<T, AppError>;

#[derive(Serialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
enum WireError {
    NotAuthenticated { service: String },
    RateLimited { service: String, retry_after_secs: u64 },
    Network { message: String },
    Api { service: String, message: String },
    Db { message: String },
    Keychain { message: String },
    Other { message: String },
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let wire = match self {
            AppError::NotAuthenticated(service) => WireError::NotAuthenticated {
                service: service.clone(),
            },
            AppError::RateLimited {
                service,
                retry_after_secs,
            } => WireError::RateLimited {
                service: service.clone(),
                retry_after_secs: *retry_after_secs,
            },
            AppError::Network(message) => WireError::Network {
                message: message.clone(),
            },
            AppError::Api { service, message } => WireError::Api {
                service: service.clone(),
                message: message.clone(),
            },
            AppError::Db(message) => WireError::Db {
                message: message.clone(),
            },
            AppError::Keychain(message) => WireError::Keychain {
                message: message.clone(),
            },
            AppError::Other(message) => WireError::Other {
                message: message.clone(),
            },
        };
        wire.serialize(serializer)
    }
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        AppError::Db(e.to_string())
    }
}

impl From<sqlx::migrate::MigrateError> for AppError {
    fn from(e: sqlx::migrate::MigrateError) -> Self {
        AppError::Db(e.to_string())
    }
}

impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        AppError::Network(e.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::Other(format!("json: {e}"))
    }
}

impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self {
        AppError::Other(e.to_string())
    }
}
