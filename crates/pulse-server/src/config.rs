use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub environment: String,
    pub database_url: String,
    pub redis_url: String,
    pub redis_key_prefix: String,
    pub admin_token: String,
    pub umami_url: Option<String>,
    pub umami_user: Option<String>,
    pub umami_pass: Option<String>,
    pub allowed_origins: Vec<String>,
    pub geoip_db_path: Option<String>,
    pub buffer_flush_interval_secs: u64,
    pub buffer_batch_size: usize,
    pub rate_limit_per_second: u32,
    pub data_retention_days: u32,
    pub cookie_secret: String,
    pub email_report_webhook_url: Option<String>,
    /// 32-byte AES-256-GCM key (base64) for encrypting BI external-DB connection
    /// strings at rest. When unset, connection strings are stored verbatim (legacy).
    pub bi_connection_kms_key: Option<[u8; 32]>,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            port: env::var("PULSE_PORT")
                .unwrap_or_else(|_| "8090".to_string())
                .parse()
                .expect("PULSE_PORT must be a valid port number"),
            environment: env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string()),
            database_url: env::var("DATABASE_URL").expect("DATABASE_URL is required"),
            redis_url: env::var("REDIS_URL").expect("REDIS_URL is required"),
            redis_key_prefix: env::var("REDIS_KEY_PREFIX")
                .unwrap_or_else(|_| "pulse_analytics:".to_string()),
            admin_token: env::var("PULSE_ADMIN_TOKEN").expect("PULSE_ADMIN_TOKEN is required"),
            umami_url: env::var("UMAMI_URL").ok(),
            umami_user: env::var("UMAMI_USER").ok(),
            umami_pass: env::var("UMAMI_PASS").ok(),
            allowed_origins: env::var("ALLOWED_ORIGINS")
                .unwrap_or_default()
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            geoip_db_path: env::var("GEOIP_DB_PATH").ok(),
            buffer_flush_interval_secs: env::var("BUFFER_FLUSH_INTERVAL_SECS")
                .unwrap_or_else(|_| "5".to_string())
                .parse()
                .unwrap_or(5),
            buffer_batch_size: env::var("BUFFER_BATCH_SIZE")
                .unwrap_or_else(|_| "500".to_string())
                .parse()
                .unwrap_or(500),
            rate_limit_per_second: env::var("RATE_LIMIT_PER_SECOND")
                .unwrap_or_else(|_| "100".to_string())
                .parse()
                .unwrap_or(100),
            data_retention_days: env::var("DATA_RETENTION_DAYS")
                .unwrap_or_else(|_| "365".to_string())
                .parse()
                .unwrap_or(365),
            cookie_secret: env::var("PULSE_COOKIE_SECRET").unwrap_or_else(|_| {
                use rand::{distr::Alphanumeric, RngExt};
                let mut rng = rand::rng();
                let secret: String = (&mut rng)
                    .sample_iter(Alphanumeric)
                    .take(64)
                    .map(char::from)
                    .collect();
                secret
            }),
            email_report_webhook_url: env::var("EMAIL_REPORT_WEBHOOK_URL").ok(),
            bi_connection_kms_key: env::var("BI_CONNECTION_KMS_KEY")
                .ok()
                .and_then(|raw| Self::parse_kms_key(&raw)),
        }
    }

    /// Decode a base64-encoded 32-byte AES-256 key. Empty means encryption is
    /// disabled; any non-empty malformed value fails closed at startup.
    fn parse_kms_key(raw: &str) -> Option<[u8; 32]> {
        use base64::Engine;
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return None;
        }
        match base64::engine::general_purpose::STANDARD.decode(trimmed) {
            Ok(bytes) if bytes.len() == 32 => {
                let mut key = [0u8; 32];
                key.copy_from_slice(&bytes);
                Some(key)
            }
            Ok(bytes) => {
                panic!(
                    "BI_CONNECTION_KMS_KEY must decode to exactly 32 bytes (got {} bytes)",
                    bytes.len()
                );
            }
            Err(err) => {
                panic!("BI_CONNECTION_KMS_KEY is not valid base64: {err}");
            }
        }
    }

    pub fn is_production(&self) -> bool {
        self.environment == "production"
    }
}

#[cfg(test)]
mod tests {
    use super::Config;
    use base64::Engine;

    #[test]
    fn parses_valid_bi_connection_kms_key() {
        let encoded = base64::engine::general_purpose::STANDARD.encode([5u8; 32]);
        assert_eq!(Config::parse_kms_key(&encoded), Some([5u8; 32]));
    }

    #[test]
    fn empty_bi_connection_kms_key_disables_encryption() {
        assert_eq!(Config::parse_kms_key(""), None);
        assert_eq!(Config::parse_kms_key("   "), None);
    }

    #[test]
    fn malformed_bi_connection_kms_key_fails_closed() {
        assert!(std::panic::catch_unwind(|| Config::parse_kms_key("not-base64")).is_err());

        let short_key = base64::engine::general_purpose::STANDARD.encode([1u8; 16]);
        assert!(std::panic::catch_unwind(|| Config::parse_kms_key(&short_key)).is_err());
    }
}
