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

    /// A client that follows no redirect (the caller reads each answer, a 3xx included). It is for requests that
    /// carry a secret in a header: a redirect could send the header to another host.
    pub fn no_redirect(user_agent: &str, timeout_seconds: u64) -> Client {
        Client::build(user_agent, 0.0, Some(0), timeout_seconds.max(1))
    }

    /// A client that follows no redirect and waits `delay_seconds` between requests. It is for requests that carry a
    /// secret in the URL (EDINET): a redirect could send the URL with the secret to another host.
    pub fn no_redirect_paced(user_agent: &str, delay_seconds: f64) -> Client {
        Client::build(user_agent, delay_seconds, Some(0), DEFAULT_TIMEOUT_SECONDS)
    }

    /// One request with the method `GET`, `DELETE` (no body) or `POST` (a JSON body). It returns the HTTP code and the
    /// body for each code, also 3xx and 4xx: the caller decides. The error text never has a header value.
    pub fn request(&mut self, method: &str, url: &str, headers: &[(&str, &str)], body: Option<&str>) -> Result<(u16, Vec<u8>), HttpError> {
        self.wait();
        let result = match (method, body) {
            ("GET", None) => {
                let mut r = self.prepare(self.agent.get(url));
                for (name, value) in headers {
                    r = r.header(*name, *value);
                }
                r.call()
            }
            ("DELETE", None) => {
                let mut r = self.prepare(self.agent.delete(url));
                for (name, value) in headers {
                    r = r.header(*name, *value);
                }
                r.call()
            }
            ("POST", Some(text)) => {
                let mut r = self.prepare(self.agent.post(url)).header("Content-Type", "application/json");
                for (name, value) in headers {
                    r = r.header(*name, *value);
                }
                r.send(text)
            }
            _ => return Err(HttpError::Transport(format!("method {method} with this body is not supported"))),
        };
        self.last = Some(Instant::now());
        match result {
            Ok(mut response) => {
                let code = response.status().as_u16();
                let mut out = Vec::new();
                response.body_mut().as_reader().read_to_end(&mut out).map_err(|e| HttpError::Transport(format!("{url}: {e}")))?;
                Ok((code, out))
            }
            Err(e) => Err(HttpError::Transport(format!("{url}: {e}"))),
        }
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

    /// A real HTTP test with a local server (no outside network): a 307 answer is not followed, and a header with a secret
    /// is sent to this one server only.
    #[test]
    fn no_redirect_client_follows_no_redirect_over_real_http() {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::{Arc, Mutex};
        static PROXY_LOCK: Mutex<()> = Mutex::new(());
        let _g = PROXY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        // The client reads the proxy variable; a local address must not go through a proxy.
        let saved: Vec<(String, Option<String>)> = ["HTTPS_PROXY", "https_proxy", "HTTP_PROXY", "http_proxy"].iter().map(|k| (k.to_string(), std::env::var(k).ok())).collect();
        for (k, _) in &saved {
            std::env::remove_var(k);
        }
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let hits = Arc::new(AtomicUsize::new(0));
        let seen = Arc::new(Mutex::new(String::new()));
        let (h2, s2) = (hits.clone(), seen.clone());
        let server = std::thread::spawn(move || {
            // Serve for a short time: the first request gets the 307; a second request would be a followed redirect.
            listener.set_nonblocking(true).unwrap();
            let end = std::time::Instant::now() + std::time::Duration::from_millis(1500);
            while std::time::Instant::now() < end {
                if let Ok((mut c, _)) = listener.accept() {
                    c.set_nonblocking(false).unwrap();
                    let mut buf = [0u8; 4096];
                    let n = c.read(&mut buf).unwrap_or(0);
                    h2.fetch_add(1, Ordering::SeqCst);
                    s2.lock().unwrap().push_str(&String::from_utf8_lossy(&buf[..n]));
                    let _ = write!(c, "HTTP/1.1 307 Temporary Redirect\r\nLocation: http://127.0.0.1:{port}/second\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
                } else {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
            }
        });
        let mut client = Client::no_redirect("test-agent", 5);
        let r = client.request("POST", &format!("http://127.0.0.1:{port}/first"), &[("X-Secret", "abc123")], Some("{}"));
        // The paced client of EDINET (the secret is in the URL) follows no redirect either.
        let mut paced = Client::no_redirect_paced("test-agent", 0.0);
        let r2 = paced.request("GET", &format!("http://127.0.0.1:{port}/first?Subscription-Key=k3y"), &[], None);
        server.join().unwrap();
        for (k, v) in saved {
            if let Some(v) = v {
                std::env::set_var(k, v);
            }
        }
        // The answer is the 307 itself (or an error), never the result of a second request.
        match r {
            Ok((code, _)) => assert_eq!(code, 307),
            Err(e) => assert!(!e.to_string().is_empty()),
        }
        match r2 {
            Ok((code, _)) => assert_eq!(code, 307),
            Err(e) => assert!(!e.to_string().is_empty()),
        }
        assert_eq!(hits.load(Ordering::SeqCst), 2, "one request for each client: no redirect was followed");
        let text = seen.lock().unwrap().to_lowercase();
        assert!(text.starts_with("post /first") && text.contains("x-secret: abc123"), "{text}");
        assert!(text.contains("get /first?subscription-key=k3y") && !text.contains("/second"), "{text}");
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
