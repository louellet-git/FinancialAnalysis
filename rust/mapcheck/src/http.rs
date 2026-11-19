//! HTTP GET with a delay between requests.
//!
//! The proxy comes from the HTTPS_PROXY environment variable. The certificates come
//! from the system store (the SSL_CERT_FILE environment variable is obeyed).

use std::io::Read;
use std::time::{Duration, Instant};

/// The SEC asks each program to give its name and a contact in the User-Agent text.
/// Set the environment variable SEC_USER_AGENT, for example
/// `SEC_USER_AGENT="FinancialAnalysis your-name your-contact"`.
pub const SEC_USER_AGENT_ENV: &str = "SEC_USER_AGENT";

pub struct Client {
    agent: ureq::Agent,
    user_agent: String,
    delay: Duration,
    last: Option<Instant>,
}

#[derive(Debug)]
pub enum HttpError {
    Status(u16, String),
    Transport(String),
}

impl std::fmt::Display for HttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpError::Status(code, url) => write!(f, "HTTP {code} for {url}"),
            HttpError::Transport(message) => write!(f, "{message}"),
        }
    }
}

impl Client {
    pub fn new(user_agent: &str, delay_seconds: f64) -> Client {
        let mut builder = ureq::AgentBuilder::new().timeout(Duration::from_secs(60));
        if let Ok(proxy) = std::env::var("HTTPS_PROXY").or_else(|_| std::env::var("https_proxy")) {
            if let Ok(proxy) = ureq::Proxy::new(&proxy) {
                builder = builder.proxy(proxy);
            }
        }
        Client {
            agent: builder.build(),
            user_agent: user_agent.to_string(),
            delay: Duration::from_secs_f64(delay_seconds.max(0.0)),
            last: None,
        }
    }

    /// A client for SEC EDGAR. The SEC permits 10 requests in one second; the delay
    /// is 0.1 s or more.
    pub fn sec(delay_seconds: f64) -> Result<Client, String> {
        let user_agent = std::env::var(SEC_USER_AGENT_ENV).map_err(|_| {
            format!(
                "set the environment variable {SEC_USER_AGENT_ENV} to the name of your program and a contact \
                 (the SEC asks for this), for example: {SEC_USER_AGENT_ENV}=\"FinancialAnalysis your-name your-contact\""
            )
        })?;
        if user_agent.trim().is_empty() {
            return Err(format!("{SEC_USER_AGENT_ENV} is empty"));
        }
        Ok(Client::new(user_agent.trim(), delay_seconds.max(0.1)))
    }

    pub fn get(&mut self, url: &str) -> Result<Vec<u8>, HttpError> {
        if let Some(last) = self.last {
            let elapsed = last.elapsed();
            if elapsed < self.delay {
                std::thread::sleep(self.delay - elapsed);
            }
        }
        let result = self
            .agent
            .get(url)
            .set("User-Agent", &self.user_agent)
            .set("Accept", "application/json, text/plain, */*")
            .call();
        self.last = Some(Instant::now());
        match result {
            Ok(response) => {
                let mut body = Vec::new();
                response
                    .into_reader()
                    .read_to_end(&mut body)
                    .map_err(|e| HttpError::Transport(format!("{url}: {e}")))?;
                Ok(body)
            }
            Err(ureq::Error::Status(code, _)) => Err(HttpError::Status(code, url.to_string())),
            Err(e) => Err(HttpError::Transport(format!("{url}: {e}"))),
        }
    }
}
