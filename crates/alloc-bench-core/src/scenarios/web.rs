//! Web scenario (SCEN-02) — in-process axum + tokio + reqwest load generator.
//!
//! Architecture: a tokio multi-thread runtime is built **once in `setup()`**
//! and stashed in `self.runtime`. Per-tick work is `runtime.block_on(...)`
//! that fires `client_workers` parallel POSTs against an in-process axum
//! `/echo` endpoint. Both client and server share the same global allocator.
//!
//! The load generator and server are co-resident so the allocator under
//! test is exercised by *both* request encoding (reqwest + serde_json) and
//! response handling (axum + tokio + serde_json). One tick = one batch of
//! `client_workers` parallel HTTP requests.
//!
//! Critical anti-patterns avoided (RESEARCH.md §Anti-Patterns):
//!   1. **Runtime per tick** — would dominate the latency histogram with
//!      tokio scheduler startup costs. Built once in `setup()` and reused.
//!   2. **Server task panic propagating into the runtime** — `axum::serve(...).await`
//!      returns `!` on success but errors on abrupt shutdown. We wrap with
//!      `unwrap_or_else(|e| eprintln!(...))` so the spawned task never panics
//!      the runtime when the scenario is dropped between ticks.
//!   3. **OS-port contention in run-all** — the listener binds `127.0.0.1:0`
//!      so the OS picks a free port; the captured `local_addr()` is used by
//!      the client. Multiple `Web` instances in one process don't fight.
//!
//! Throughput unit: per CONTEXT.md decision, the Run record's
//! `scenario.unit = "req_per_s"`. The CLI dispatcher (`run_web`) supplies
//! that override; the scenario itself returns `name() == "web"` and
//! `allocations_per_tick() == client_workers`.

use std::net::SocketAddr;
use std::time::Duration;

use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};

use crate::harness::{Scenario, SinkValue};

#[derive(Debug, Clone, Serialize)]
pub struct WebConfig {
    pub server_workers: usize,
    pub client_workers: usize,
    pub seed: u64,
}

impl WebConfig {
    /// Reject malformed configs at construction time.
    pub fn validated(self) -> anyhow::Result<Self> {
        anyhow::ensure!(
            self.server_workers >= 1,
            "server_workers must be >= 1 (got {})",
            self.server_workers
        );
        anyhow::ensure!(
            self.client_workers >= 1,
            "client_workers must be >= 1 (got {})",
            self.client_workers
        );
        Ok(self)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Address {
    street: String,
    city: String,
    state: String,
    zip: String,
    country: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct UserProfile {
    id: u64,
    username: String,
    email: String,
    full_name: String,
    address: Address,
    tags: Vec<String>,
    metadata: serde_json::Map<String, serde_json::Value>,
    created_at: String,
    last_login: String,
    notes: String,
}

/// Generate a deterministic ~1KB UserProfile from a seeded RNG. Heap shape:
/// 10 String fields (varied lengths), one Vec<String> of 5 entries, one
/// nested Address (5 Strings), one serde_json::Map with 3 nested keys.
/// Total serialized JSON ~1KB matches the CONTEXT.md target.
fn make_user_profile(rng: &mut SmallRng) -> UserProfile {
    let id: u64 = rng.gen();
    // Strings of deterministic but varied length — RESEARCH.md §Web Payload.
    let username = format!("user_{:08x}", rng.gen::<u32>());
    let email = format!(
        "{}@{}.example.com",
        "x".repeat(rng.gen_range(8..=20)),
        "y".repeat(rng.gen_range(4..=10))
    );
    let full_name = format!(
        "{} {}",
        "First".repeat(rng.gen_range(1..=4)),
        "Last".repeat(rng.gen_range(1..=4))
    );
    let address = Address {
        street: format!("{} Main St", rng.gen::<u16>()),
        city: "City".repeat(rng.gen_range(1..=4)),
        state: "ST".to_string(),
        zip: format!("{:05}", rng.gen_range(10000..=99999_u32)),
        country: "Country".to_string(),
    };
    // 5 tags, ~12 bytes each.
    let tags: Vec<String> = (0..5)
        .map(|i| format!("tag_{i:02}_{:04x}", rng.gen::<u16>()))
        .collect();
    let mut metadata = serde_json::Map::new();
    metadata.insert(
        "source".to_string(),
        serde_json::Value::String("benchmark".repeat(rng.gen_range(1..=4))),
    );
    metadata.insert(
        "score".to_string(),
        serde_json::Value::Number(serde_json::Number::from(rng.gen::<u32>())),
    );
    metadata.insert(
        "labels".to_string(),
        serde_json::Value::Array(vec![
            serde_json::Value::String("a".repeat(rng.gen_range(4..=12))),
            serde_json::Value::String("b".repeat(rng.gen_range(4..=12))),
            serde_json::Value::String("c".repeat(rng.gen_range(4..=12))),
        ]),
    );
    let created_at = "2026-05-18T10:00:00Z".to_string();
    let last_login = "2026-05-18T11:00:00Z".to_string();
    // 256-byte free-form notes (CONTEXT.md target).
    let notes = "lorem_".repeat(42);

    UserProfile {
        id,
        username,
        email,
        full_name,
        address,
        tags,
        metadata,
        created_at,
        last_login,
        notes,
    }
}

/// Echo handler — tiny mutation defeats `echo == identity` DCE. The handler
/// also exercises the server-side allocator: deserialise body → mutate →
/// reserialise into the response body.
async fn echo_handler(axum::Json(p): axum::Json<UserProfile>) -> axum::Json<UserProfile> {
    let mut p = p;
    p.id = p.id.wrapping_add(1);
    axum::Json(p)
}

pub struct Web {
    cfg: WebConfig,
    runtime: Option<tokio::runtime::Runtime>,
    server_addr: Option<SocketAddr>,
    client: Option<reqwest::Client>,
    /// CR-04 (Phase-2 review): per-tick counter mixed into the payload
    /// RNG seed so each tick generates a different `UserProfile`. Without
    /// this, every tick produced the *exact same* serialised body, letting
    /// the HTTP layer (reqwest connection pool, hyper request encoder,
    /// axum handler) potentially cache identical bodies and elide the
    /// per-payload allocation work the benchmark is supposed to measure.
    /// `wrapping_add` so a multi-million-tick soak never panics on
    /// overflow. Mirrors `FragmentationSoak::rng` (which advances across
    /// ticks for the same reason).
    tick_seq: u64,
}

impl Web {
    pub fn new(cfg: WebConfig) -> Self {
        Self {
            cfg,
            runtime: None,
            server_addr: None,
            client: None,
            tick_seq: 0,
        }
    }
}

impl Scenario for Web {
    fn name(&self) -> &'static str {
        "web"
    }

    fn config_json(&self) -> serde_json::Value {
        serde_json::to_value(&self.cfg).unwrap_or_else(|_| serde_json::json!({}))
    }

    fn allocations_per_tick(&self) -> u64 {
        // Approximation: one POST per client worker, each generating a
        // payload + a response payload. Aggregator can multiply
        // ticks_per_s * client_workers to get req/s.
        self.cfg.client_workers as u64
    }

    /// Build the runtime, bind the listener, spawn the server, build the
    /// client. All four artefacts survive into `tick()` via `self`.
    fn setup(&mut self) -> anyhow::Result<()> {
        use anyhow::Context;

        // RESEARCH.md §Pitfall 1: runtime built ONCE here, never per tick.
        // WR-01 (Phase-2 review): wrap each ? with .context(...) so a
        // run-all failure surfaces a meaningful error string in the JSON
        // \`error\` field rather than a bare \`std::io::Error\` like
        // "Address already in use (os error 98)".
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(self.cfg.server_workers)
            .enable_all()
            .build()
            .context("web scenario: build tokio runtime")?;

        // Bind on the runtime — TcpListener::bind is async.
        let actual_addr = runtime
            .block_on(async {
                let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
                    .await
                    .context("web scenario: bind 127.0.0.1:0")?;
                let addr = listener
                    .local_addr()
                    .context("web scenario: read listener local_addr")?;
                // We need the listener to outlive this block_on, so move it
                // into the spawned server task. Capture addr first.
                let app =
                    axum::Router::new().route("/echo", axum::routing::post(echo_handler));
                // Spawn the server fire-and-forget on the runtime so the bind
                // happens here-and-now and the serve loop runs in the background
                // for the rest of the scenario's life.
                tokio::spawn(async move {
                    // RESEARCH.md §A6: axum::serve(...).await returns ! on
                    // success and Err on shutdown. Swallow any error so a
                    // dropped runtime doesn't propagate into a runtime panic.
                    axum::serve(listener, app)
                        .await
                        .unwrap_or_else(|e| eprintln!("axum::serve exited: {e}"));
                });
                Ok::<_, anyhow::Error>(addr)
            })
            .context("web scenario: setup listener + server task")?;

        // reqwest client uses the runtime context implicitly via
        // tokio::runtime::Handle::current() inside its async fns.
        let client = runtime
            .block_on(async {
                reqwest::Client::builder()
                    .pool_max_idle_per_host(self.cfg.client_workers)
                    .timeout(Duration::from_secs(10))
                    .build()
            })
            .context("web scenario: build reqwest client")?;

        self.runtime = Some(runtime);
        self.server_addr = Some(actual_addr);
        self.client = Some(client);
        Ok(())
    }

    /// One tick = `client_workers` parallel POSTs to the in-process server.
    /// Each task allocates a payload, sends, awaits the response, and
    /// the response payload joins a Vec that the harness `black_box`es.
    fn tick(&mut self) -> Box<dyn SinkValue> {
        let runtime = self.runtime.as_ref().expect("setup() not called");
        let client = self.client.clone().expect("setup() not called");
        let url = format!(
            "http://{}/echo",
            self.server_addr.expect("setup() not called")
        );
        let client_workers = self.cfg.client_workers;
        // CR-04 (Phase-2 review): mix the tick counter into the seed so
        // each tick produces a *different* UserProfile. Without this, the
        // payload was byte-identical every tick — letting the HTTP layer
        // potentially cache and elide the per-payload allocation work
        // the benchmark targets. The seed-and-counter pattern mirrors
        // MPSC's `cfg.seed.wrapping_add(p as u64)` (channels.rs:239),
        // which derives per-thread RNG seeds from the same base.
        let seed = self.cfg.seed.wrapping_add(self.tick_seq);
        self.tick_seq = self.tick_seq.wrapping_add(1);
        let payload = make_user_profile(&mut SmallRng::seed_from_u64(seed));

        // WR-02 (Phase-2 review): each per-task HTTP round-trip used
        // .expect("client.send failed") / .expect("response.json failed"),
        // turning a transient HTTP error (slow-spawn, connection-reset
        // mid-shutdown) into a panic that takes down the WHOLE web
        // scenario via tokio::JoinHandle. A single dropped request
        // should not invalidate the other 99% of ticks. Each task now
        // returns an Option<UserProfile> — on transport / decode failure
        // we eprintln + return None instead of panicking. We still
        // black_box the count of failures so the optimizer can't elide
        // the failure path itself.
        let (responses, failed_count): (Vec<UserProfile>, u64) = runtime.block_on(async move {
            let mut handles = Vec::with_capacity(client_workers);
            for _ in 0..client_workers {
                let client = client.clone();
                let url = url.clone();
                let payload = payload.clone();
                handles.push(tokio::spawn(async move {
                    let resp = match client.post(&url).json(&payload).send().await {
                        Ok(r) => r,
                        Err(e) => {
                            eprintln!("web tick: send failed (recorded as None): {e}");
                            return None;
                        }
                    };
                    match resp.json::<UserProfile>().await {
                        Ok(p) => Some(p),
                        Err(e) => {
                            eprintln!("web tick: response.json failed (recorded as None): {e}");
                            None
                        }
                    }
                }));
            }
            let mut out = Vec::with_capacity(client_workers);
            let mut failed = 0u64;
            for h in handles {
                // Phase-1 CR-02: panics inside the spawned task still
                // propagate via resume_unwind — only HTTP-layer errors
                // are downgraded to None. A panic indicates a logic bug
                // (e.g., invariant violation in the handler) and MUST
                // surface so the harness records `status: "failed"`.
                match h.await {
                    Ok(Some(resp)) => out.push(resp),
                    Ok(None) => failed = failed.wrapping_add(1),
                    Err(e) if e.is_panic() => std::panic::resume_unwind(e.into_panic()),
                    Err(e) => panic!("tokio task failed: {e}"),
                }
            }
            (out, failed)
        });

        // Defeat DCE on both the success-vec AND the failure count so
        // the optimizer cannot elide either path.
        std::hint::black_box(failed_count);
        Box::new(std::hint::black_box(responses))
    }

    /// Default — runtime drops naturally with `&mut self` drop, terminating
    /// the spawned server task.
    fn teardown(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(server_workers: usize, client_workers: usize) -> WebConfig {
        WebConfig {
            server_workers,
            client_workers,
            seed: 1,
        }
    }

    #[test]
    fn validated_rejects_zero_server_workers() {
        let err = cfg(0, 1).validated().unwrap_err();
        assert!(err.to_string().contains("server_workers must be >= 1"));
    }

    #[test]
    fn validated_rejects_zero_client_workers() {
        let err = cfg(1, 0).validated().unwrap_err();
        assert!(err.to_string().contains("client_workers must be >= 1"));
    }

    #[test]
    fn validated_accepts_well_formed_config() {
        assert!(cfg(2, 4).validated().is_ok());
    }

    #[test]
    fn make_user_profile_is_deterministic_for_seed() {
        let mut rng_a = SmallRng::seed_from_u64(42);
        let mut rng_b = SmallRng::seed_from_u64(42);
        let a = make_user_profile(&mut rng_a);
        let b = make_user_profile(&mut rng_b);
        assert_eq!(a.username, b.username);
        assert_eq!(a.email, b.email);
        assert_eq!(a.tags, b.tags);
    }

    #[test]
    fn make_user_profile_serialises_to_non_trivial_json() {
        let mut rng = SmallRng::seed_from_u64(7);
        let p = make_user_profile(&mut rng);
        let json = serde_json::to_string(&p).expect("serialize UserProfile");
        // CONTEXT.md target: ~1KB. Sanity-check we're in the right ballpark.
        assert!(
            json.len() >= 600,
            "expected at least 600 bytes of serialised JSON, got {}",
            json.len()
        );
    }

    #[test]
    fn allocations_per_tick_matches_client_workers() {
        let s = Web::new(cfg(2, 4));
        assert_eq!(s.allocations_per_tick(), 4);
    }

    #[test]
    fn tick_smoke_does_not_panic() {
        // 1 server worker + 1 client worker keeps the smoke fast and
        // avoids port-bind contention. Validates setup() builds the
        // runtime, binds the listener, builds the client, and one tick
        // round-trips a request without panicking.
        let c = cfg(1, 1).validated().unwrap();
        let mut s = Web::new(c);
        s.setup().expect("setup");
        let _ = s.tick();
    }
}
