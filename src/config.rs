use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub bind_addr: String,
    pub discord_webhook_url: Option<String>,
    pub apprise_url: Option<String>,
    pub pushover_app_token: Option<String>,
    pub pushover_user_key: Option<String>,
    pub steamgriddb_api_key: Option<String>,
    pub scheduler_tick_secs: u64,
}

impl Config {
    pub fn from_env() -> Self {
        let database_url = env::var("DATABASE_URL")
            .expect("DATABASE_URL must be set");

        Config {
            database_url,
            bind_addr: env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:3079".to_string()),
            discord_webhook_url: env::var("DISCORD_WEBHOOK_URL").ok(),
            apprise_url: env::var("APPRISE_URL").ok(),
            pushover_app_token: env::var("PUSHOVER_APP_TOKEN").ok(),
            pushover_user_key: env::var("PUSHOVER_USER_KEY").ok(),
            steamgriddb_api_key: env::var("STEAMGRIDDB_API_KEY").ok(),
            scheduler_tick_secs: env::var("DISCOPROWL_SCHEDULER_TICK_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(60),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[serial_test::serial]
    #[test]
    fn config_requires_database_url() {
        std::env::remove_var("DATABASE_URL");
        let result = std::panic::catch_unwind(Config::from_env);
        assert!(result.is_err());
    }

    #[serial_test::serial]
    #[test]
    fn config_parses_optional_channels() {
        std::env::set_var("DATABASE_URL", "/tmp/test.db");
        std::env::set_var("DISCORD_WEBHOOK_URL", "https://discord.com/api/webhooks/test");
        std::env::remove_var("APPRISE_URL");
        std::env::remove_var("PUSHOVER_APP_TOKEN");
        std::env::remove_var("PUSHOVER_USER_KEY");
        let cfg = Config::from_env();
        assert!(cfg.discord_webhook_url.is_some());
        assert!(cfg.apprise_url.is_none());
    }
}
