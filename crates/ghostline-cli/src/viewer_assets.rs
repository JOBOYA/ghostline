use rust_embed::Embed;

#[derive(Embed)]
#[folder = "../../viewer/dist/"]
pub struct ViewerAssets;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_viewer_assets_has_index() {
        let index = ViewerAssets::get("index.html");
        assert!(index.is_some(), "index.html not found in embedded assets");
        let has_js = ViewerAssets::iter().any(|f| f.ends_with(".js"));
        assert!(has_js, "no JS file found in embedded assets");
    }
}
