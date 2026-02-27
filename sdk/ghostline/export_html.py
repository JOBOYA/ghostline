"""Export a .ghostline file as a self-contained HTML file with embedded viewer."""

import base64
import importlib.resources
import os

# Viewer dist files are bundled relative to the package
_VIEWER_DIR = os.path.join(os.path.dirname(__file__), "..", "..", "viewer", "dist")


def _find_viewer_assets() -> tuple[str, str]:
    """Locate the built viewer JS and CSS files."""
    assets_dir = os.path.join(_VIEWER_DIR, "assets")
    if not os.path.isdir(assets_dir):
        raise FileNotFoundError(
            f"Viewer dist not found at {assets_dir}. Run 'cd viewer && npm run build' first."
        )

    js_file = css_file = None
    for f in os.listdir(assets_dir):
        if f.endswith(".js"):
            js_file = os.path.join(assets_dir, f)
        elif f.endswith(".css"):
            css_file = os.path.join(assets_dir, f)

    if not js_file or not css_file:
        raise FileNotFoundError("Viewer JS/CSS not found in dist/assets/")

    return js_file, css_file


def export_html(ghostline_path: str, output_path: str | None = None) -> str:
    """Export a .ghostline file as a standalone HTML file.

    The output is a single HTML file that embeds:
    - The Ghostline viewer (React app) as inline JS/CSS
    - The .ghostline binary data as a base64 blob

    Anyone can open it in a browser — no server needed.

    Args:
        ghostline_path: Path to the .ghostline file.
        output_path: Destination HTML path. Defaults to <file>.html.

    Returns:
        Path to the generated HTML file.
    """
    if output_path is None:
        output_path = ghostline_path.removesuffix(".ghostline") + ".html"

    # Read the .ghostline binary
    with open(ghostline_path, "rb") as f:
        data_b64 = base64.b64encode(f.read()).decode("ascii")

    # Read viewer assets
    js_path, css_path = _find_viewer_assets()
    with open(js_path, "r") as f:
        js_content = f.read()
    with open(css_path, "r") as f:
        css_content = f.read()

    filename = os.path.basename(ghostline_path)

    html = f"""<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>Ghostline — {filename}</title>
  <link rel="preconnect" href="https://fonts.googleapis.com" />
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin />
  <link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600&family=JetBrains+Mono:wght@400;500&display=swap" rel="stylesheet" />
  <style>{css_content}</style>
</head>
<body>
  <div id="root"></div>
  <script id="ghostline-data" type="application/octet-stream" data-filename="{filename}">{data_b64}</script>
  <script type="module">{js_content}</script>
</body>
</html>"""

    with open(output_path, "w") as f:
        f.write(html)

    return output_path
