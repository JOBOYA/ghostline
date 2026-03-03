pub const LOGO: &str = r#"
   ██████╗ ██╗  ██╗ ██████╗ ███████╗████████╗██╗     ██╗███╗   ██╗███████╗
  ██╔════╝ ██║  ██║██╔═══██╗██╔════╝╚══██╔══╝██║     ██║████╗  ██║██╔════╝
  ██║  ███╗███████║██║   ██║███████╗   ██║   ██║     ██║██╔██╗ ██║█████╗
  ██║   ██║██╔══██║██║   ██║╚════██║   ██║   ██║     ██║██║╚██╗██║██╔══╝
  ╚██████╔╝██║  ██║╚██████╔╝███████║   ██║   ███████╗██║██║ ╚████║███████╗
   ╚═════╝ ╚═╝  ╚═╝ ╚═════╝ ╚══════╝   ╚═╝   ╚══════╝╚═╝╚═╝  ╚═══╝╚══════╝
"#;

pub fn print_startup(proxy_port: u16, viewer_port: u16) {
    println!("{}", LOGO);
    println!("  v2.0.0 — Deterministic replay for AI agents.\n");
    println!("  ✓ Proxy listening on  http://localhost:{}", proxy_port);
    println!("  ✓ Viewer serving on   http://localhost:{}", viewer_port);
    println!();
    println!("┌────────────────────────────────────────────────────────┐");
    println!("│ Ready! Open a new terminal and run:                    │");
    println!("│                                                        │");
    println!("│   export ANTHROPIC_BASE_URL=http://localhost:{}       │", proxy_port);
    println!("│   claude                                               │");
    println!("│                                                        │");
    println!("│ Or use:  ghostline run claude                          │");
    println!("│                                                        │");
    println!("│ All API calls will be captured automatically.          │");
    println!("│ View them live at http://localhost:{}                 │", viewer_port);
    println!("└────────────────────────────────────────────────────────┘\n");
}

pub fn print_frame(index: usize, latency_ms: u64, size_bytes: usize) {
    let now = chrono::Local::now().format("%H:%M:%S");
    println!(
        "[{}] ● FRAME {} | {}ms | {:.1}KB",
        now,
        index,
        latency_ms,
        size_bytes as f64 / 1024.0
    );
}
