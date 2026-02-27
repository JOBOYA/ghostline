use ghostline_core::{Frame, GhostlineWriter, Header};
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};
use std::io::BufWriter;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;

use crate::viewer_server::FrameSender;

type Writer = GhostlineWriter<BufWriter<std::fs::File>>;

struct ProxyState {
    target: String,
    client: reqwest::Client,
    writer: Option<Writer>,
    frame_count: usize,
    frame_tx: Option<FrameSender>,
    shared_frame_count: Arc<std::sync::atomic::AtomicUsize>,
}

async fn handle(
    req: Request<Body>,
    state: Arc<Mutex<ProxyState>>,
) -> Result<Response<Body>, hyper::Error> {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let path = uri.path_and_query().map(|p| p.as_str()).unwrap_or("/");
    let headers = req.headers().clone();
    let body_bytes = hyper::body::to_bytes(req.into_body()).await?;

    let s = state.lock().await;
    let url = format!("{}{}", s.target, path);
    let mut builder = s.client.request(
        reqwest::Method::from_bytes(method.as_str().as_bytes()).unwrap(),
        &url,
    );
    for (name, value) in headers.iter() {
        match name.as_str() {
            "host" | "connection" | "transfer-encoding" => continue,
            n => builder = builder.header(n, value.as_bytes()),
        }
    }
    builder = builder.body(body_bytes.to_vec());
    drop(s);

    let start = Instant::now();
    let resp = match builder.send().await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[proxy] ERROR: {}", e);
            return Ok(Response::builder().status(502).body(Body::from(format!("{}", e))).unwrap());
        }
    };
    let latency_ms = start.elapsed().as_millis() as u64;

    let status = resp.status();
    let resp_headers = resp.headers().clone();
    let resp_bytes = resp.bytes().await.unwrap_or_default();

    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap()
        .as_millis() as u64;
    let frame = Frame::new(body_bytes.to_vec(), resp_bytes.to_vec(), latency_ms, now_ms);

    let mut s = state.lock().await;
    if let Some(ref mut w) = s.writer {
        if let Err(e) = w.append(&frame) {
            eprintln!("[proxy] write error: {}", e);
        }
    }
    s.frame_count += 1;
    let fc = s.frame_count;
    s.shared_frame_count.store(fc, std::sync::atomic::Ordering::Relaxed);

    // Broadcast frame to WebSocket viewers
    if let Some(ref tx) = s.frame_tx {
        let frame_json = serde_json::json!({
            "index": fc,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "request_size": body_bytes.len(),
            "response_size": resp_bytes.len(),
            "latency_ms": latency_ms,
        });
        let _ = tx.send(frame_json.to_string());
    }
    drop(s);

    crate::banner::print_frame(fc, latency_ms, resp_bytes.len());

    let mut rb = Response::builder().status(status.as_u16());
    for (name, value) in resp_headers.iter() {
        match name.as_str() {
            "transfer-encoding" | "connection" => {}
            n => rb = rb.header(n, value.as_bytes()),
        }
    }
    rb = rb.header("x-ghostline-proxy", "true");
    Ok(rb.body(Body::from(resp_bytes.to_vec())).unwrap())
}

pub async fn run_proxy(
    port: u16,
    out: PathBuf,
    target: String,
    frame_tx: Option<FrameSender>,
    shared_frame_count: Arc<std::sync::atomic::AtomicUsize>,
) -> anyhow::Result<()> {
    std::fs::create_dir_all(&out)?;

    let now = chrono::Utc::now();
    let filename = format!("{}-{}.ghostline", now.format("%Y%m%d-%H%M%S"), uuid::Uuid::new_v4());
    let filepath = out.join(&filename);

    let file = BufWriter::new(std::fs::File::create(&filepath)?);
    let header = Header { started_at: now.timestamp_millis() as u64, git_sha: None, parent_run_id: None, fork_at_step: None };
    let writer = GhostlineWriter::new(file, &header)?;

    let client = reqwest::Client::builder().no_proxy().build()?;
    let target_clean = target.trim_end_matches('/').to_string();

    let state = Arc::new(Mutex::new(ProxyState {
        target: target_clean.clone(),
        client,
        writer: Some(writer),
        frame_count: 0,
        frame_tx,
        shared_frame_count,
    }));

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    eprintln!("ghostline transparent proxy");
    eprintln!("  target: {}", target_clean);
    eprintln!("  output: {}", filepath.display());
    eprintln!("  listen: http://{}", addr);
    eprintln!();
    eprintln!("Usage: ANTHROPIC_BASE_URL=http://localhost:{} your-command", port);
    eprintln!("Ctrl+C to stop and finalize the .ghostline file.");

    let state2 = state.clone();
    let make_svc = make_service_fn(move |_| {
        let state = state2.clone();
        async move { Ok::<_, hyper::Error>(service_fn(move |req| handle(req, state.clone()))) }
    });

    let server = Server::bind(&addr).serve(make_svc);
    let graceful = server.with_graceful_shutdown(async {
        tokio::signal::ctrl_c().await.ok();
        eprintln!("\n[proxy] shutting down...");
    });
    graceful.await?;

    // Finalize
    let mut s = state.lock().await;
    let fc = s.frame_count;
    if let Some(w) = s.writer.take() {
        w.finish()?;
    }
    eprintln!("[proxy] recorded {} frames to {}", fc, filepath.display());
    Ok(())
}
