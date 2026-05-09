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
        }
    }

    pub fn is_production(&self) -> bool {
        self.environment == "production"
    }
}
