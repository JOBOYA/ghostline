use ghostline_core::{Frame, GhostlineReader};
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server, StatusCode};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Pre-loaded frame cache keyed by request hash.
struct ReplayCache {
    frames: HashMap<[u8; 32], Frame>,
    hits: u64,
    misses: u64,
}

impl ReplayCache {
    fn lookup(&mut self, hash: &[u8; 32]) -> Option<&Frame> {
        if let Some(frame) = self.frames.get(hash) {
            self.hits += 1;
            Some(frame)
        } else {
            self.misses += 1;
            None
        }
    }
}

/// Load all frames from a .ghostline file into a hash map.
fn load_cache(path: &str) -> io::Result<ReplayCache> {
    let mut reader = GhostlineReader::open(path)?;
    let count = reader.frame_count();
    let mut frames = HashMap::with_capacity(count);

    for i in 0..count {
        let frame = reader.get_frame(i)?;
        frames.insert(frame.request_hash, frame);
    }

    Ok(ReplayCache {
        frames,
        hits: 0,
        misses: 0,
    })
}

async fn handle_request(
    req: Request<Body>,
    cache: Arc<Mutex<ReplayCache>>,
) -> Result<Response<Body>, hyper::Error> {
    let method = req.method().clone();
    let uri = req.uri().clone();

    // GET /status — health check
    if method == hyper::Method::GET && uri.path() == "/status" {
        let c = cache.lock().await;
        let body = serde_json::json!({
            "ok": true,
            "cached_frames": c.frames.len(),
            "hits": c.hits,
            "misses": c.misses,
        });
        return Ok(Response::builder()
            .status(200)
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap());
    }

    // For all other requests: hash the body, look up cached response
    let body_bytes = hyper::body::to_bytes(req.into_body()).await?;

    let mut hasher = Sha256::new();
    hasher.update(&body_bytes);
    let hash: [u8; 32] = hasher.finalize().into();

    let mut c = cache.lock().await;
    match c.lookup(&hash) {
        Some(frame) => {
            eprintln!(
                "[replay] HIT {} {} → {}ms latency, {} bytes",
                method,
                uri,
                frame.latency_ms,
                frame.response_bytes.len()
            );
            // Serve the cached response bytes directly
            // The response_bytes contain the raw response body as captured
            Ok(Response::builder()
                .status(200)
                .header("content-type", "application/json")
                .header("x-ghostline-replay", "true")
                .header("x-ghostline-latency-ms", frame.latency_ms.to_string())
                .body(Body::from(frame.response_bytes.clone()))
                .unwrap())
        }
        None => {
            eprintln!(
                "[replay] MISS {} {} — hash {}",
                method,
                uri,
                hex::encode(&hash[..8])
            );
            let body = serde_json::json!({
                "error": "no cached response for this request",
                "request_hash": hex::encode(&hash),
            });
            Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .header("content-type", "application/json")
                .header("x-ghostline-replay", "miss")
                .body(Body::from(body.to_string()))
                .unwrap())
        }
    }
}

pub async fn run_replay_server(file: &str, port: u16) -> anyhow::Result<()> {
    let cache = Arc::new(Mutex::new(load_cache(file)?));
    let frame_count = cache.lock().await.frames.len();

    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    eprintln!("ghostline replay proxy");
    eprintln!("  file:   {}", file);
    eprintln!("  frames: {}", frame_count);
    eprintln!("  listen: http://{}", addr);
    eprintln!();
    eprintln!("Point your AI client at http://{}/ to replay cached responses.", addr);
    eprintln!("GET /status for cache stats. Ctrl+C to stop.");

    let make_svc = make_service_fn(move |_conn| {
        let cache = cache.clone();
        async move {
            Ok::<_, hyper::Error>(service_fn(move |req| {
                handle_request(req, cache.clone())
            }))
        }
    });

    Server::bind(&addr).serve(make_svc).await?;

    Ok(())
}
