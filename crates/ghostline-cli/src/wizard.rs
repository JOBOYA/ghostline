use crate::config::Config;
use dialoguer::{Confirm, Input};

pub fn run_wizard() -> anyhow::Result<Config> {
    println!("\n First time? Let's set up.\n");

    let token: String = Input::new()
        .with_prompt("Enter your Claude Code token\n  (Run `claude config get apiKey` in another terminal to get it)\n  Token")
        .interact_text()?;

    // Base64 obfuscate for storage
    let encoded = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        token.as_bytes(),
    );

    println!(" ✓ Token saved");

    let scrub = Confirm::new()
        .with_prompt("Scrub secrets from recordings? (recommended)")
        .default(true)
        .interact()?;

    let auto_open = Confirm::new()
        .with_prompt("Auto-open browser when starting?")
        .default(true)
        .interact()?;

    let mut cfg = Config::default();
    cfg.auth.claude_token = Some(encoded);
    cfg.recording.scrub = scrub;
    cfg.viewer.auto_open_browser = auto_open;

    let path = Config::config_path();
    cfg.save(&path)?;
    println!(" ✓ Config saved to {}", path.display());

    Ok(cfg)
}
