//! HTTP GET with a delay between requests.
//!
//! The proxy comes from the HTTPS_PROXY environment variable. The certificates come
//! from the system store (the SSL_CERT_FILE environment variable is obeyed).

use std::io::Read;
use std::time::{Duration, Instant};

/// The SEC asks each program to give its name and a contact in the User-Agent text.
/// The contact is in the file `sec_contact.toml` in the repo root (a project address, public on
/// purpose). The environment variable SEC_USER_AGENT replaces it, for example
/// `SEC_USER_AGENT="FinancialAnalysis your-name your-contact"`.
pub const SEC_USER_AGENT_ENV: &str = "SEC_USER_AGENT";

/// The name of the contact file in the repo root.
pub const SEC_CONTACT_FILE: &str = "sec_contact.toml";

/// The environment variable that names the repo root.
pub const REPO_ENV: &str = "FA_REPO";

/// The SEC contact text and the place that it came from.
#[derive(Debug, Clone, PartialEq)]
pub struct SecContact {
    pub user_agent: String,
    pub source: ContactSource,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ContactSource {
    /// The environment variable `SEC_USER_AGENT`.
    Variable,
    /// The file `sec_contact.toml` (the path of the file).
    File(std::path::PathBuf),
}

impl SecContact {
    /// A short text for a status line. It never has the contact text.
    pub fn describe_source(&self) -> String {
        match &self.source {
            ContactSource::Variable => SEC_USER_AGENT_ENV.to_string(),
            ContactSource::File(_) => format!("file {SEC_CONTACT_FILE}"),
        }
    }
}

/// The SEC contact: the variable `SEC_USER_AGENT` if it is set and not empty, else the file
/// `sec_contact.toml` (see `contact_search_dirs`).
pub fn sec_user_agent() -> Result<SecContact, String> {
    let value = std::env::var(SEC_USER_AGENT_ENV).ok();
    find_contact(&contact_search_dirs(), value.as_deref())
}

/// The folders where the file is looked for, in order: the folder of `FA_REPO`; the current folder
/// and each parent folder; the repo root known when the program was built.
pub fn contact_search_dirs() -> Vec<std::path::PathBuf> {
    let mut dirs: Vec<std::path::PathBuf> = Vec::new();
    if let Some(repo) = std::env::var_os(REPO_ENV).filter(|v| !v.is_empty()) {
        dirs.push(repo.into());
    }
    if let Ok(cwd) = std::env::current_dir() {
        dirs.extend(cwd.ancestors().map(|p| p.to_path_buf()));
    }
    dirs.push(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join(".."));
    dirs
}

/// The pure part of `sec_user_agent`: the value of the variable (if any) and the folders to search.
pub fn find_contact(dirs: &[std::path::PathBuf], variable: Option<&str>) -> Result<SecContact, String> {
    if let Some(v) = variable.map(str::trim).filter(|v| !v.is_empty()) {
        return Ok(SecContact { user_agent: v.to_string(), source: ContactSource::Variable });
    }
    for dir in dirs {
        let path = dir.join(SEC_CONTACT_FILE);
        if path.is_file() {
            let user_agent = read_contact_file(&path)?;
            return Ok(SecContact { user_agent, source: ContactSource::File(path) });
        }
    }
    Err(format!(
        "no SEC contact found. The SEC asks for a name and a contact in each request. Do one of these: \
         put the file {SEC_CONTACT_FILE} (key user_agent) in the repo root, or set the environment \
         variable {SEC_USER_AGENT_ENV} (for example {SEC_USER_AGENT_ENV}=\"FinancialAnalysis your-name your-contact\")"
    ))
}

fn read_contact_file(path: &std::path::Path) -> Result<String, String> {
    let shown = path.display();
    let text = std::fs::read_to_string(path).map_err(|e| format!("{shown}: {e}"))?;
    let table: toml::Table = text.parse().map_err(|e| format!("{shown}: not a valid TOML file: {e}"))?;
    let value = table
        .get("user_agent")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| format!("{shown}: the key user_agent is missing or empty"))?;
    if !value.contains('@') {
        return Err(format!("{shown}: the key user_agent has no contact address (no @ in the text)"));
    }
    Ok(value.to_string())
}

/// The time limit of one request when the caller gives none.
pub const DEFAULT_TIMEOUT_SECONDS: u64 = 60;

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
        Client::build(user_agent, delay_seconds, None, DEFAULT_TIMEOUT_SECONDS)
    }

    /// The same as `new`, with the time limit of one request (the whole request, in seconds).
    pub fn with_timeout(user_agent: &str, delay_seconds: f64, timeout_seconds: u64) -> Client {
        Client::build(user_agent, delay_seconds, None, timeout_seconds.max(1))
    }

    /// A client for a web form that needs a session. It keeps the cookies, sends
    /// them again, and does not follow redirects (the caller reads each step).
    pub fn session(user_agent: &str, delay_seconds: f64) -> Client {
        let mut client = Client::build(user_agent, delay_seconds, Some(0), DEFAULT_TIMEOUT_SECONDS);
        client.cookies = Some(Vec::new());
        client
    }

    fn build(user_agent: &str, delay_seconds: f64, redirects: Option<u32>, timeout_seconds: u64) -> Client {
        // `http_status_as_error(false)`: the code reads the status itself (a 3xx answer
        // is a normal step in a form session).
        // Certificates: the store of the operating system (on Linux this follows the standard
        // variable SSL_CERT_FILE), not the list that is built into the library. A network
        // with its own certificate authority (a company proxy, the proxy of a cloud session
        // that adds a secret to a request) then works with no change of the program.
        let tls = ureq::tls::TlsConfig::builder().root_certs(ureq::tls::RootCerts::PlatformVerifier).build();
        let mut config = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(timeout_seconds)))
            .tls_config(tls)
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
        let contact = sec_user_agent()?;
        Ok(Client::new(&contact.user_agent, delay_seconds.max(0.1)))
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// A new empty folder for one test.
    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("mapcheck-contact-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_contact(dir: &std::path::Path, text: &str) {
        std::fs::write(dir.join(SEC_CONTACT_FILE), text).unwrap();
    }

    #[test]
    fn the_variable_comes_before_the_file() {
        let dir = temp("order");
        write_contact(&dir, "user_agent = \"Project project@example.org\"\n");
        let c = find_contact(&[dir.clone()], Some("  Me me@example.org  ")).unwrap();
        assert_eq!(c.user_agent, "Me me@example.org");
        assert_eq!(c.source, ContactSource::Variable);
        assert_eq!(c.describe_source(), "SEC_USER_AGENT");
        // An empty variable does not count: the file is used.
        let c = find_contact(&[dir.clone()], Some("   ")).unwrap();
        assert_eq!(c.user_agent, "Project project@example.org");
        assert_eq!(c.source, ContactSource::File(dir.join(SEC_CONTACT_FILE)));
        assert_eq!(c.describe_source(), "file sec_contact.toml");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_file_is_found_in_a_parent_folder_and_the_first_folder_wins() {
        let root = temp("parent");
        let deep = root.join("a").join("b");
        std::fs::create_dir_all(&deep).unwrap();
        write_contact(&root, "user_agent = \"Root root@example.org\"\n");
        let dirs: Vec<PathBuf> = deep.ancestors().map(|p| p.to_path_buf()).collect();
        assert_eq!(find_contact(&dirs, None).unwrap().user_agent, "Root root@example.org");
        // A file in a nearer folder is used first.
        write_contact(&root.join("a"), "user_agent = \"Near near@example.org\"\n");
        assert_eq!(find_contact(&dirs, None).unwrap().user_agent, "Near near@example.org");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_bad_file_is_an_error_that_names_the_file() {
        let dir = temp("bad");
        let path = dir.join(SEC_CONTACT_FILE);
        for text in [
            "user_agent = \"\"\n",
            "other = \"x\"\n",
            "user_agent = 5\n",
            "user_agent = \"no address here\"\n",
            "user_agent = \n",
        ] {
            write_contact(&dir, text);
            let e = find_contact(&[dir.clone()], None).unwrap_err();
            assert!(e.contains(&path.display().to_string()), "{text}: {e}");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn no_variable_and_no_file_says_the_two_ways() {
        let dir = temp("none");
        let e = find_contact(&[dir.clone()], None).unwrap_err();
        assert!(e.contains("sec_contact.toml") && e.contains("SEC_USER_AGENT"), "{e}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_repo_file_is_good() {
        // The file of this repo (found from the build-time root) must give a valid contact.
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..");
        let c = find_contact(&[root], None).unwrap();
        assert!(c.user_agent.contains('@'));
    }
}
