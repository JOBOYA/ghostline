"""Tests for the scrubbing layer."""

import json
from ghostline.scrub import ScrubConfig, scrub_bytes, scrub_text


def test_scrub_openai_key():
    text = '{"api_key": "sk-proj-abc123def456ghi789jkl012mno"}'
    result = scrub_text(text)
    assert "sk-proj-" not in result
    assert "[REDACTED_OPENAI_KEY]" in result


def test_scrub_anthropic_key():
    text = '{"key": "sk-ant-api03-abcdefghijklmnopqrstuvwx"}'
    result = scrub_text(text)
    assert "sk-ant-" not in result
    assert "[REDACTED_ANTHROPIC_KEY]" in result


def test_scrub_stripe_keys():
    # Build the test key dynamically to avoid GitHub push protection
    prefix = "sk_" + "live_"
    key = prefix + "x" * 30
    text = '{"sk": "' + key + '"}'
    result = scrub_text(text)
    assert prefix not in result
    assert "[REDACTED_STRIPE_KEY]" in result


def test_scrub_github_token():
    text = '{"token": "ghp_FAKE00TEST00TOKEN00VALUE00PLACEHOLDER00"}'
    result = scrub_text(text)
    assert "ghp_" not in result
    assert "[REDACTED_GITHUB_TOKEN]" in result


def test_scrub_email():
    text = '{"user": "joseph.boyadjian@gmail.com"}'
    result = scrub_text(text)
    assert "@gmail.com" not in result
    assert "[REDACTED_EMAIL]" in result


def test_scrub_bearer_token():
    text = 'Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.abc123'
    result = scrub_text(text)
    assert "eyJhbGci" not in result
    assert "Bearer [REDACTED_TOKEN]" in result


def test_scrub_bytes_roundtrip():
    data = b'{"key": "sk-ant-api03-secretkey1234567890abcdef"}'
    result = scrub_bytes(data)
    assert b"sk-ant-" not in result
    assert b"[REDACTED_ANTHROPIC_KEY]" in result


def test_scrub_custom_strings():
    config = ScrubConfig(custom_strings=[("my-secret-value", "[REDACTED]")])
    text = "the password is my-secret-value here"
    result = scrub_text(text, config)
    assert "my-secret-value" not in result
    assert "[REDACTED]" in result


def test_scrub_extra_patterns():
    config = ScrubConfig(extra_patterns=[
        (r"CUSTOM-[A-Z0-9]{10}", "[REDACTED_CUSTOM]"),
    ])
    text = "token: CUSTOM-ABCDEF1234"
    result = scrub_text(text, config)
    assert "CUSTOM-ABCDEF1234" not in result
    assert "[REDACTED_CUSTOM]" in result


def test_scrub_no_emails_option():
    config = ScrubConfig(redact_emails=False)
    text = "contact: user@example.com"
    result = scrub_text(text, config)
    assert "user@example.com" in result  # Not redacted


def test_scrub_preserves_clean_data():
    text = '{"model": "claude-3-5-sonnet", "messages": [{"role": "user", "content": "hello"}]}'
    result = scrub_text(text)
    assert result == text  # Nothing to scrub


def test_scrub_multiple_keys_in_one_string():
    text = json.dumps({
        "key1": "sk-ant-api03-firstkey12345678901234",
        "key2": "sk-proj-secondkey90abcdef12345678",
        "email": "test@example.org",
    })
    result = scrub_text(text)
    assert "sk-ant-" not in result
    assert "sk-proj-" not in result
    assert "@example.org" not in result


def test_recorder_with_scrub():
    """Integration test: recorder scrubs before writing."""
    import tempfile
    from pathlib import Path
    from ghostline.recorder import GhostlineRecorder
    from ghostline.format import GhostlineReader

    with tempfile.NamedTemporaryFile(suffix=".ghostline", delete=False) as f:
        tmp = Path(f.name)

    recorder = GhostlineRecorder(tmp, scrub=True)
    recorder.start()
    req = b'{"api_key": "sk-ant-api03-secretkey1234567890abcdef", "prompt": "hello"}'
    resp = b'{"text": "hi from api", "meta": {"email": "user@test.com"}}'
    recorder.capture(req, resp, 100)
    recorder.stop()

    # Read back and verify scrubbing
    with open(tmp, "rb") as f:
        reader = GhostlineReader(f)
        assert reader.frame_count == 1
        frame = reader.get_frame(0)
    assert b"sk-ant-" not in frame.request_bytes
    assert b"[REDACTED_ANTHROPIC_KEY]" in frame.request_bytes
    assert b"@test.com" not in frame.response_bytes
    assert b"[REDACTED_EMAIL]" in frame.response_bytes
    # Non-sensitive data preserved
    assert b"hello" in frame.request_bytes
    assert b"hi from api" in frame.response_bytes

    tmp.unlink()
