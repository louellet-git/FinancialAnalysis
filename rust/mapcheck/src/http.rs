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
    /// Session cookies (name, value). `None` for a client without a session.
    cookies: Option<Vec<(String, String)>>,
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
        Client::build(user_agent, delay_seconds, None)
    }

    /// A client for a web form that needs a session. It keeps the cookies, sends
    /// them again, and does not follow redirects (the caller reads each step).
    pub fn session(user_agent: &str, delay_seconds: f64) -> Client {
        let mut client = Client::build(user_agent, delay_seconds, Some(0));
        client.cookies = Some(Vec::new());
        client
    }

    fn build(user_agent: &str, delay_seconds: f64, redirects: Option<u32>) -> Client {
        // `http_status_as_error(false)`: the code reads the status itself (a 3xx answer
        // is a normal step in a form session).
        let mut config = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(60)))
            .http_status_as_error(false);
        // Version 2 followed 5 redirects; keep that rule.
        config = config.max_redirects(redirects.unwrap_or(5));
        if let Ok(proxy) = std::env::var("HTTPS_PROXY").or_else(|_| std::env::var("https_proxy")) {
            if let Ok(proxy) = ureq::Proxy::new(&proxy) {
                config = config.proxy(Some(proxy));
            }
        }
        Client {
            agent: config.build().into(),
            user_agent: user_agent.to_string(),
            delay: Duration::from_secs_f64(delay_seconds.max(0.0)),
            last: None,
            cookies: None,
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
        self.get_with_headers(url, &[])
    }

    /// GET with more request headers (for example `Authorization`). The error text never has a header value.
    pub fn get_with_headers(&mut self, url: &str, headers: &[(&str, &str)]) -> Result<Vec<u8>, HttpError> {
        self.wait();
        let mut request = self.prepare(self.agent.get(url));
        for (name, value) in headers {
            request = request.header(*name, *value);
        }
        let result = request.call();
        self.finish(result, url)
    }

    /// POST a form (`application/x-www-form-urlencoded`). Use it with `Client::session`.
    pub fn post_form(&mut self, url: &str, fields: &[(&str, &str)]) -> Result<Vec<u8>, HttpError> {
        self.wait();
        let request = self.prepare(self.agent.post(url));
        let result = request.send_form(fields.iter().copied());
        self.finish(result, url)
    }

    /// POST a JSON text (`application/json`).
    pub fn post_json(&mut self, url: &str, body: &str) -> Result<Vec<u8>, HttpError> {
        self.wait();
        let request = self.prepare(self.agent.post(url)).header("Content-Type", "application/json");
        let result = request.send(body);
        self.finish(result, url)
    }

    fn wait(&mut self) {
        if let Some(last) = self.last {
            let elapsed = last.elapsed();
            if elapsed < self.delay {
                std::thread::sleep(self.delay - elapsed);
            }
        }
    }

    fn prepare<B>(&self, request: ureq::RequestBuilder<B>) -> ureq::RequestBuilder<B> {
        let request = request
            .header("User-Agent", &self.user_agent)
            .header("Accept", "application/json, text/plain, */*");
        match &self.cookies {
            Some(jar) if !jar.is_empty() => {
                let header: Vec<String> = jar.iter().map(|(k, v)| format!("{k}={v}")).collect();
                request.header("Cookie", &header.join("; "))
            }
            _ => request,
        }
    }

    fn remember(&mut self, response: &ureq::http::Response<ureq::Body>) {
        if let Some(jar) = self.cookies.as_mut() {
            for line in response.headers().get_all("set-cookie") {
                let line = String::from_utf8_lossy(line.as_bytes()).to_string();
                if let Some((name, value)) = line.split(';').next().and_then(|p| p.split_once('=')) {
                    jar.retain(|(k, _)| k != name.trim());
                    jar.push((name.trim().to_string(), value.trim().to_string()));
                }
            }
        }
    }

    fn finish(
        &mut self,
        result: Result<ureq::http::Response<ureq::Body>, ureq::Error>,
        url: &str,
    ) -> Result<Vec<u8>, HttpError> {
        self.last = Some(Instant::now());
        match result {
            Ok(mut response) => {
                self.remember(&response);
                let code = response.status().as_u16();
                if code >= 400 {
                    return Err(HttpError::Status(code, url.to_string()));
                }
                let mut body = Vec::new();
                response
                    .body_mut()
                    .as_reader()
                    .read_to_end(&mut body)
                    .map_err(|e| HttpError::Transport(format!("{url}: {e}")))?;
                Ok(body)
            }
            Err(e) => Err(HttpError::Transport(format!("{url}: {e}"))),
        }
    }
}
