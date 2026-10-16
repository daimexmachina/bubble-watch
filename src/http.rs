//! Shared HTTP client.
//!
//! Design constraints that were established by probing these hosts directly:
//!
//!   * SEC EDGAR *requires* a descriptive User-Agent and rate-limits to ~10
//!     requests/second. We default to ~5/s and identify the tool honestly.
//!   * Yahoo's chart endpoint rejects default library agents, so we send a
//!     browser-like one.
//!   * FRED refused this host entirely during probing (HTTP/2 stream errors and
//!     timeouts) after a modest burst of requests. We force HTTP/1.1, back off,
//!     and treat exhaustion as a *reported gap*, never as an error that aborts
//!     the run.
//!
//! A failed fetch is data about coverage. It is never silently swallowed and it
//! never overwrites a value from a previous successful run.

use std::cell::{Cell, RefCell};
use std::io::Read;
use std::time::{Duration, Instant};

pub struct Fetcher {
    agent: ureq::Agent,
    retries: u32,
    min_gap: Duration,
    last_call: Cell<Option<Instant>>,
    pub log: RefCell<Vec<String>>,
    pub offline: bool,
    cache_dir: Option<std::path::PathBuf>,
    /// Hosts that have exhausted their retries this run. Any further request to
    /// one of them fails immediately instead of burning another timeout cycle.
    ///
    /// This is a circuit breaker, and it exists for a specific observed reason:
    /// FRED stopped answering this host mid-probe. Without the breaker, an
    /// optional source that is simply down costs 3 retries x 3 series x 45 s of
    /// dead waiting and the tool appears to hang. An optional source must never
    /// be able to stall the run.
    dead_hosts: RefCell<std::collections::HashSet<String>>,
}

/// Query parameters whose VALUES must never be logged or embedded in output.
/// An API key in a URL is the classic leak path: it lands in request logs,
/// error messages, provenance fields and rendered reports.
const SENSITIVE_PARAMS: &[&str] = &[
    "api_key",
    "apikey",
    "key",
    "token",
    "access_token",
    "auth",
    "password",
    "secret",
];

/// Redact the values of sensitive query parameters in a URL.
///
/// Applied to every URL that is logged OR stored as provenance, so a keyed
/// request can be fully auditable ("we called this endpoint") without the
/// credential ever being written anywhere.
pub fn redact_url(url: &str) -> String {
    let Some((base, query)) = url.split_once('?') else {
        return url.to_string();
    };
    let cleaned: Vec<String> = query
        .split('&')
        .map(|pair| match pair.split_once('=') {
            Some((k, _)) if SENSITIVE_PARAMS.contains(&k.to_ascii_lowercase().as_str()) => {
                format!("{}=***", k)
            }
            _ => pair.to_string(),
        })
        .collect();
    format!("{}?{}", base, cleaned.join("&"))
}

/// Why a fetch failed. The distinction matters: a missing concept is a normal,
/// deterministic answer, while a transport failure is a real gap.
#[derive(Debug, Clone, PartialEq)]
pub enum FetchError {
    /// HTTP 404. The resource simply does not exist for this entity — a normal
    /// outcome for an XBRL tag a filer does not use. Never retried, never trips
    /// the breaker, and callers translate it into "absent" rather than "failed".
    NotFound,
    /// HTTP 400/401/403. The request itself is wrong — a rejected or missing API
    /// key, a malformed parameter. Deterministic: retrying cannot help, and it
    /// must NOT trip the host breaker, because the host is healthy and a
    /// credential problem should not suppress the remaining series (which may
    /// well succeed via a different transport).
    Rejected(String),
    /// Transport error, 5xx, or 429. Retried, and trips the breaker when the
    /// retry budget is exhausted.
    Unavailable(String),
}

impl FetchError {
    pub fn message(&self) -> String {
        match self {
            FetchError::NotFound => "404 not found".to_string(),
            FetchError::Rejected(m) => format!("request rejected: {}", m),
            FetchError::Unavailable(m) => m.clone(),
        }
    }
    pub fn is_not_found(&self) -> bool {
        matches!(self, FetchError::NotFound)
    }
    /// True when retrying and circuit-breaking are both pointless.
    pub fn is_deterministic(&self) -> bool {
        matches!(self, FetchError::NotFound | FetchError::Rejected(_))
    }
}

impl std::fmt::Display for FetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message())
    }
}

impl Fetcher {
    pub fn new(offline: bool, cache_dir: Option<std::path::PathBuf>) -> Self {
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(15))
            .timeout(Duration::from_secs(45))
            // Some hosts (observed: FRED) behave badly on HTTP/2 from this
            // network; HTTP/1.1 is markedly more reliable here.
            .try_proxy_from_env(false)
            .build();
        Fetcher {
            agent,
            retries: 3,
            // Yahoo returned HTTP 429 during probing when requests were fired
            // faster than this. 350 ms keeps the whole run under roughly three
            // requests per second, which was observed to be tolerated.
            min_gap: Duration::from_millis(350),
            last_call: Cell::new(None),
            log: RefCell::new(Vec::new()),
            offline,
            cache_dir,
            dead_hosts: RefCell::new(std::collections::HashSet::new()),
        }
    }

    fn throttle(&self) {
        if let Some(prev) = self.last_call.get() {
            let elapsed = prev.elapsed();
            if elapsed < self.min_gap {
                std::thread::sleep(self.min_gap - elapsed);
            }
        }
        self.last_call.set(Some(Instant::now()));
    }

    /// Fetch, serving a cached copy while it is fresh — even ONLINE.
    ///
    /// WHY THIS EXISTS. Online mode otherwise refetches every URL on every run. That is right for a
    /// live market series and wrong for a source that changes weekly or monthly, and on 2026-10-16 it
    /// broke a channel outright: measured against CourtListener, this host is throttled at roughly
    /// **five requests per window** while the model makes **ten** (five docket queries x two date
    /// windows each). The last five were starved with HTTP 429, and `distress_events` sits 7th-10th, so
    /// it returned a GAP on every run while `opposition_pressure` — which needs only five — looked
    /// healthy. Nothing was wrong with the new queries; they simply never got a turn.
    ///
    /// A TTL means those ten requests are spent ONCE and a same-day re-run costs nothing. The threshold
    /// is per CALL, not global: a price series wants no cache at all, a court docket count is fine for a
    /// day. The cached file's mtime IS the fetch time, so no extra bookkeeping is needed.
    pub fn get_with_ttl(
        &self,
        url: &str,
        user_agent: &str,
        ttl: std::time::Duration,
    ) -> Result<String, String> {
        if !self.offline {
            if let Some(p) = self.cache_path(url) {
                let fresh = std::fs::metadata(&p)
                    .and_then(|m| m.modified())
                    .map(|t| t.elapsed().map(|age| age < ttl).unwrap_or(false))
                    .unwrap_or(false);
                if fresh {
                    if let Ok(bytes) = std::fs::read(&p) {
                        if !bytes.is_empty() {
                            self.log.borrow_mut().push(format!(
                                "CACHE fresh ({}) {}",
                                ttl.as_secs(),
                                redact_url(url)
                            ));
                            return Ok(String::from_utf8_lossy(&bytes).to_string());
                        }
                    }
                }
            }
        }
        self.get(url, user_agent)
    }

    /// Fetch binary content with a TTL, for archives that change slowly.
    pub fn get_bytes_with_ttl(
        &self,
        url: &str,
        user_agent: &str,
        ttl: std::time::Duration,
    ) -> Result<Vec<u8>, String> {
        if !self.offline {
            if let Some(p) = self.cache_path(url) {
                let fresh = std::fs::metadata(&p)
                    .and_then(|m| m.modified())
                    .map(|t| t.elapsed().map(|age| age < ttl).unwrap_or(false))
                    .unwrap_or(false);
                if fresh {
                    if let Ok(bytes) = std::fs::read(&p) {
                        if !bytes.is_empty() {
                            self.log.borrow_mut().push(format!(
                                "CACHE fresh ({}) {}",
                                ttl.as_secs(),
                                redact_url(url)
                            ));
                            return Ok(bytes);
                        }
                    }
                }
            }
        }
        self.get_bytes(url, user_agent)
    }

    fn cache_path(&self, url: &str) -> Option<std::path::PathBuf> {
        let dir = self.cache_dir.as_ref()?;
        // Small FNV-1a hash keeps filenames filesystem-safe and stable.
        let mut h: u64 = 0xcbf29ce484222325;
        for b in url.as_bytes() {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        Some(dir.join(format!("{:016x}.body", h)))
    }

    /// Convenience wrapper that flattens the error into a message.
    pub fn get(&self, url: &str, user_agent: &str) -> Result<String, String> {
        self.get_raw(url, user_agent, self.retries)
            .map_err(|e| e.message())
    }

    /// Core fetch with an explicit retry budget, preserving the error kind.
    pub fn get_raw(&self, url: &str, user_agent: &str, retries: u32) -> Result<String, FetchError> {
        self.get_raw_bytes(url, user_agent, retries).and_then(|b| {
            String::from_utf8(b).map_err(|e| {
                FetchError::Unavailable(format!("response was not valid UTF-8: {}", e))
            })
        })
    }

    /// Binary variant, for archives. The Fed Z.1 release is an 8MB zip, so this
    /// cannot go through the `String` path; everything else — the cache, the
    /// circuit breaker, the retry policy, the 404-is-absence rule — behaves
    /// identically.
    pub fn get_bytes(&self, url: &str, user_agent: &str) -> Result<Vec<u8>, String> {
        self.get_raw_bytes(url, user_agent, self.retries)
            .map_err(|e| e.message())
    }

    fn get_raw_bytes(
        &self,
        url: &str,
        user_agent: &str,
        retries: u32,
    ) -> Result<Vec<u8>, FetchError> {
        let host = host_of(url);

        // Offline mode never touches the network; it is a cache read or a gap.
        if self.offline {
            if let Some(p) = self.cache_path(url) {
                if let Ok(s) = std::fs::read(&p) {
                    self.log
                        .borrow_mut()
                        .push(format!("CACHE hit {}", redact_url(url)));
                    return Ok(s);
                }
            }
            return Err(FetchError::Unavailable(format!(
                "offline: no cached copy of {}",
                redact_url(url)
            )));
        }

        // Circuit breaker: a host already declared dead this run gets no more
        // attempts. Fail fast and say so, rather than hanging again.
        if self.dead_hosts.borrow().contains(&host) {
            self.log
                .borrow_mut()
                .push(format!("BREAKER open, skipped {}", redact_url(url)));
            return Err(FetchError::Unavailable(format!(
                "host {} already failed earlier in this run (circuit breaker open)",
                host
            )));
        }

        let mut last_err = String::from("no attempt made");
        for attempt in 1..=retries.max(1) {
            self.throttle();
            match self
                .agent
                .get(url)
                .set("User-Agent", user_agent)
                .set("Accept", "*/*")
                .call()
            {
                Ok(resp) => {
                    // Read as bytes so the same path serves text and archives; the
                    // text callers convert afterwards.
                    let mut body: Vec<u8> = Vec::new();
                    resp.into_reader()
                        .read_to_end(&mut body)
                        .map_err(|e| FetchError::Unavailable(e.to_string()))?;
                    if body.is_empty() {
                        last_err = "empty response body".into();
                        continue;
                    }
                    if let Some(p) = self.cache_path(url) {
                        if let Some(parent) = p.parent() {
                            let _ = std::fs::create_dir_all(parent);
                        }
                        let _ = std::fs::write(&p, &body);
                    }
                    self.log
                        .borrow_mut()
                        .push(format!("OK({}) {}", attempt, redact_url(url)));
                    return Ok(body);
                }
                Err(ureq::Error::Status(404, _)) => {
                    // Deterministic absence. Report it immediately: no retry,
                    // and never trip the breaker for a whole host because one
                    // concept does not exist.
                    self.log
                        .borrow_mut()
                        .push(format!("404 (absent) {}", redact_url(url)));
                    return Err(FetchError::NotFound);
                }
                Err(ureq::Error::Status(code, resp))
                    if code == 400 || code == 401 || code == 403 =>
                {
                    // A rejected request — typically a bad/missing API key.
                    // Deterministic and host-independent: do not retry, and do
                    // not trip the breaker, or one credential problem would
                    // suppress every other series on that host.
                    let detail = resp
                        .into_string()
                        .map(|b| redact_url(b.trim()))
                        .unwrap_or_default();
                    let msg = if detail.is_empty() {
                        format!("HTTP {}", code)
                    } else {
                        format!(
                            "HTTP {} — {}",
                            code,
                            detail.chars().take(200).collect::<String>()
                        )
                    };
                    self.log.borrow_mut().push(format!(
                        "REJECTED({}) {} :: {}",
                        code,
                        redact_url(url),
                        msg
                    ));
                    return Err(FetchError::Rejected(msg));
                }
                Err(ureq::Error::Status(429, _)) => {
                    // RATE LIMITED — the one status where retrying at the ordinary inter-request gap
                    // makes things worse. Measured against CourtListener (2026-10-16): five requests
                    // succeed, the sixth and every request after it are refused with 429, and a retry
                    // 30 seconds later is STILL refused. So the ordinary `min_gap` is far too short to
                    // clear this, and a naive retry loop just re-spends the same refused requests.
                    //
                    // Sleeping for a fixed, generous interval is deliberately cruder than exponential
                    // backoff: the goal is to outlast a fixed window, not to probe for its edge.
                    last_err = "HTTP 429 rate limited".into();
                    self.log.borrow_mut().push(format!(
                        "RATE-LIMITED({}) {} :: sleeping {}s before retry",
                        attempt,
                        redact_url(url),
                        RATE_LIMIT_BACKOFF_SECS
                    ));
                    std::thread::sleep(std::time::Duration::from_secs(RATE_LIMIT_BACKOFF_SECS));
                }
                Err(e) => {
                    // ureq embeds the URL in its error display, so the key would
                    // otherwise reach the log through this path too.
                    last_err = redact_url(&format!("{}", e));
                    self.log.borrow_mut().push(format!(
                        "FAIL({}) {} :: {}",
                        attempt,
                        redact_url(url),
                        last_err
                    ));
                    if attempt < retries.max(1) {
                        // Exponential backoff, bounded. A 429 or a stream error
                        // needs a real pause, not a token one.
                        let backoff = Duration::from_millis(750 * (1 << (attempt - 1)) as u64);
                        std::thread::sleep(backoff);
                    }
                }
            }
        }

        // Exhausted: trip the breaker for this host so sibling requests to the
        // same source fail immediately.
        self.dead_hosts.borrow_mut().insert(host);
        Err(FetchError::Unavailable(last_err))
    }
}

/// Extract the host from a URL for circuit-breaker bookkeeping.
fn host_of(url: &str) -> String {
    url.split("://")
        .nth(1)
        .and_then(|rest| rest.split('/').next())
        .unwrap_or(url)
        .to_string()
}

/// Honest, descriptive UA for SEC endpoints (they require contact info).
/// SEC requires a descriptive User-Agent that INCLUDES A CONTACT ADDRESS.
///
/// Measured on this host, 2026-09-20: the previous value — which named the tool and a
/// purpose but carried NO contact address — now receives **HTTP 403** from
/// `www.sec.gov/Archives/...`, with a body reading "Your Request Originates from an
/// Undeclared Automated Tool". A browser User-Agent is refused too (also 403), so this is
/// not bot-detection evasion: SEC's policy is that automated clients must be identifiable
/// AND reachable, and a UA without a contact does not qualify.
///
/// Adding a contact address returns 200 for the identical request. The address is a
/// deliberately non-routable placeholder so no real mailbox is published in a public repo;
/// anyone deploying this should substitute their own.
pub const UA_EDGAR: &str = "bubble-watch/0.1 (research; contact: research@example.invalid)";
pub const UA_WEB: &str =
    "Mozilla/5.0 (X11; Linux aarch64) AppleWebKit/537.36 Chrome/120 Safari/537.36";

/// How long to sleep after an HTTP 429 before retrying.
///
/// 45 seconds, chosen from measurement rather than symmetry: against CourtListener on 2026-10-16 the
/// fifth request succeeded, the sixth was refused, and a retry more than 30 seconds later was STILL
/// refused — so the window is longer than 30s and anything shorter than that is wasted. Sleeping out a
/// fixed window is deliberately cruder than exponential backoff; the goal is to outlast the throttle,
/// not to probe for its edge.
pub const RATE_LIMIT_BACKOFF_SECS: u64 = 45;
pub const UA_FRED: &str = "Mozilla/5.0 (compatible; bubble-watch/0.1)";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_api_key_values() {
        let u = "https://api.stlouisfed.org/fred/series/observations?series_id=DGS10&file_type=json&api_key=abcdef1234567890abcdef1234567890";
        let r = redact_url(u);
        assert!(!r.contains("abcdef1234567890"), "key leaked: {}", r);
        assert!(r.contains("api_key=***"), "key not masked: {}", r);
        // The rest of the URL must survive so the call stays auditable.
        assert!(r.contains("series_id=DGS10"));
        assert!(r.contains("file_type=json"));
    }

    #[test]
    fn redacts_other_sensitive_param_names_case_insensitively() {
        for p in [
            "api_key", "API_KEY", "token", "apikey", "secret", "password",
        ] {
            let u = format!("https://x.test/p?a=1&{}={}", p, "SUPERSECRETVALUE");
            let r = redact_url(&u);
            assert!(!r.contains("SUPERSECRETVALUE"), "leaked via {}: {}", p, r);
        }
    }

    #[test]
    fn leaves_urls_without_query_or_secrets_untouched() {
        assert_eq!(
            redact_url("https://data.sec.gov/api/xbrl/companyconcept/x.json"),
            "https://data.sec.gov/api/xbrl/companyconcept/x.json"
        );
        assert_eq!(
            redact_url("https://query1.finance.yahoo.com/v8/finance/chart/RSP?range=1y"),
            "https://query1.finance.yahoo.com/v8/finance/chart/RSP?range=1y"
        );
    }

    #[test]
    fn redacts_a_key_embedded_in_an_error_string() {
        // ureq embeds the request URL in its error Display, which is how a key
        // would otherwise reach the log on the failure path.
        let err = "https://api.stlouisfed.org/fred/series?api_key=SEKRET123: Network Error";
        let r = redact_url(err);
        assert!(
            !r.contains("SEKRET123"),
            "key leaked through error text: {}",
            r
        );
    }

    #[test]
    fn host_extraction_ignores_query() {
        assert_eq!(
            host_of("https://api.stlouisfed.org/fred/series?api_key=x"),
            "api.stlouisfed.org"
        );
    }

    #[test]
    fn rejected_errors_are_deterministic_but_not_absence() {
        let e = FetchError::Rejected("HTTP 400 — api_key is not set".into());
        assert!(e.is_deterministic(), "a 400 must not be retried");
        assert!(!e.is_not_found(), "a 400 is not the same as absent");
        assert!(e.message().contains("rejected"));
        // A transport failure, by contrast, may be worth retrying.
        let t = FetchError::Unavailable("timed out".into());
        assert!(!t.is_deterministic());
    }

    #[test]
    fn redacted_rejection_text_still_explains_itself() {
        let rej = FetchError::Rejected("HTTP 400 — Variable api_key is not set".into());
        assert!(rej.message().contains("api_key is not set"));
    }

    // ---- the TTL cache ----------------------------------------------------

    fn ttl_fetcher(dir: &std::path::Path) -> Fetcher {
        // online=true, so this is the ONLINE path exercising a cache read — the whole point.
        Fetcher::new(false, Some(dir.to_path_buf()))
    }

    fn scratch(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("bw-ttl-{}-{}", name, std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn cache_was_served(f: &Fetcher) -> bool {
        f.log.borrow().iter().any(|l| l.contains("CACHE fresh"))
    }

    #[test]
    fn a_fresh_cache_entry_is_served_without_touching_the_network() {
        // THE FIX FOR THE STARVATION BUG. Without this, every online run re-spends its whole request
        // budget, and a source throttled below the number of requests the model makes starves whichever
        // queries come last. Asserted via the LOG rather than the return value so the test never
        // performs a network call.
        let dir = scratch("fresh");
        let f = ttl_fetcher(&dir);
        let url = "https://example.invalid/ttl-fresh";
        std::fs::write(f.cache_path(url).unwrap(), b"CACHED-BODY").unwrap();

        let _ = f.get_with_ttl(url, "ua", std::time::Duration::from_secs(3600));
        assert!(
            cache_was_served(&f),
            "a freshly written entry must be served from cache, not refetched: {:?}",
            f.log.borrow()
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_STALE_cache_entry_is_not_served() {
        // TTL of zero: everything is stale. Without this, the freshness comparison could be inverted or
        // dropped entirely and the test above would still pass — which is the failure mode that would
        // make this cache hide live data instead of protecting a rate limit.
        let dir = scratch("stale");
        let f = ttl_fetcher(&dir);
        let url = "https://example.invalid/ttl-stale";
        std::fs::write(f.cache_path(url).unwrap(), b"CACHED-BODY").unwrap();

        let _ = f.get_with_ttl(url, "ua", std::time::Duration::from_secs(0));
        assert!(
            !cache_was_served(&f),
            "a stale entry must NOT be served: {:?}",
            f.log.borrow()
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn an_EMPTY_cached_file_is_never_served_as_if_it_were_a_response() {
        // An empty body is a silent hole, not a response. Serving it would turn a truncated write into a
        // parse failure somewhere far away, or worse into a zero.
        let dir = scratch("empty");
        let f = ttl_fetcher(&dir);
        let url = "https://example.invalid/ttl-empty";
        std::fs::write(f.cache_path(url).unwrap(), b"").unwrap();

        let _ = f.get_with_ttl(url, "ua", std::time::Duration::from_secs(3600));
        assert!(
            !cache_was_served(&f),
            "an empty cache file must be treated as a miss: {:?}",
            f.log.borrow()
        );
        std::fs::remove_dir_all(&dir).ok();
    }
}
