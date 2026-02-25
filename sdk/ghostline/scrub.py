"""Scrub sensitive data from recorded .ghostline frames.

Redacts API keys, tokens, and PII from request and response bytes
before they are written to disk. Configurable via pattern lists.
"""

import re
from dataclasses import dataclass, field

# Common patterns for sensitive data
_DEFAULT_PATTERNS: list[tuple[str, str]] = [
    # Anthropic (must be before generic sk-)
    (r"sk-ant-[A-Za-z0-9_-]{20,}", "[REDACTED_ANTHROPIC_KEY]"),
    # OpenAI (must be before generic sk-)
    (r"sk-proj-[A-Za-z0-9_-]{20,}", "[REDACTED_OPENAI_KEY]"),
    # Stripe
    (r"sk_live_[A-Za-z0-9_-]{20,}", "[REDACTED_STRIPE_KEY]"),
    (r"sk_test_[A-Za-z0-9_-]{20,}", "[REDACTED_STRIPE_KEY]"),
    (r"pk_live_[A-Za-z0-9_-]{20,}", "[REDACTED_STRIPE_KEY]"),
    (r"pk_test_[A-Za-z0-9_-]{20,}", "[REDACTED_STRIPE_KEY]"),
    # API keys (generic fallback — after specific patterns)
    (r"sk-[A-Za-z0-9_-]{20,}", "[REDACTED_API_KEY]"),
    # AWS
    (r"AKIA[A-Z0-9]{16}", "[REDACTED_AWS_KEY]"),
    # GitHub
    (r"ghp_[A-Za-z0-9]{36}", "[REDACTED_GITHUB_TOKEN]"),
    (r"gho_[A-Za-z0-9]{36}", "[REDACTED_GITHUB_TOKEN]"),
    (r"github_pat_[A-Za-z0-9_]{22,}", "[REDACTED_GITHUB_TOKEN]"),
    # Generic bearer tokens
    (r"Bearer\s+[A-Za-z0-9_\-.]{20,}", "Bearer [REDACTED_TOKEN]"),
    # Email addresses
    (r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}", "[REDACTED_EMAIL]"),
    # Base64-encoded secrets (long base64 strings often used for auth)
    (r"(?:api[_-]?key|token|secret|password|authorization)[\"']?\s*[:=]\s*[\"']?([A-Za-z0-9+/=]{32,})",
     "[REDACTED_SECRET]"),
]


@dataclass
class ScrubConfig:
    """Configuration for the scrubbing layer.

    Attributes:
        patterns: List of (regex, replacement) tuples. Defaults include
            common API key formats, emails, and bearer tokens.
        extra_patterns: Additional patterns appended to defaults.
        redact_emails: Whether to redact email addresses (default True).
        custom_strings: Exact strings to redact (e.g., known API keys).
    """

    patterns: list[tuple[str, str]] = field(default_factory=list)
    extra_patterns: list[tuple[str, str]] = field(default_factory=list)
    redact_emails: bool = True
    custom_strings: list[tuple[str, str]] = field(default_factory=list)

    def __post_init__(self):
        if not self.patterns:
            base = list(_DEFAULT_PATTERNS)
            if not self.redact_emails:
                base = [(p, r) for p, r in base if r != "[REDACTED_EMAIL]"]
            self.patterns = base

    @property
    def all_patterns(self) -> list[tuple[str, str]]:
        return self.patterns + self.extra_patterns


def _compile_patterns(config: ScrubConfig) -> list[tuple[re.Pattern, str]]:
    """Compile regex patterns for a config."""
    return [(re.compile(p), r) for p, r in config.all_patterns]


def scrub_bytes(data: bytes, config: ScrubConfig | None = None) -> bytes:
    """Scrub sensitive data from bytes.

    Args:
        data: Raw bytes (typically JSON-encoded request/response).
        config: Scrub configuration. Uses defaults if None.

    Returns:
        Scrubbed bytes with sensitive values replaced.
    """
    if config is None:
        config = ScrubConfig()

    text = data.decode("utf-8", errors="replace")

    # Apply regex patterns
    compiled = _compile_patterns(config)
    for pattern, replacement in compiled:
        text = pattern.sub(replacement, text)

    # Apply exact string replacements
    for original, replacement in config.custom_strings:
        text = text.replace(original, replacement)

    return text.encode("utf-8")


def scrub_text(text: str, config: ScrubConfig | None = None) -> str:
    """Scrub sensitive data from a string.

    Convenience wrapper around scrub_bytes for string inputs.
    """
    return scrub_bytes(text.encode("utf-8"), config).decode("utf-8")
