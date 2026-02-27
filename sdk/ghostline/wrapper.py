"""Wrap AI client libraries to intercept API calls for recording/replay."""

import json
import time
import hashlib
from functools import wraps

from ghostline.recorder import GhostlineRecorder
from ghostline.replayer import GhostlineReplayer

# Thread-local active session
_active_recorder: GhostlineRecorder | None = None
_active_replayer: GhostlineReplayer | None = None


def set_recorder(recorder: GhostlineRecorder | None):
    global _active_recorder
    _active_recorder = recorder


def set_replayer(replayer: GhostlineReplayer | None):
    global _active_replayer
    _active_replayer = replayer


def wrap(client):
    """Wrap an Anthropic or OpenAI client to intercept API calls.

    Returns the same client with patched methods. When a recorder or
    replayer is active (via context managers), calls are intercepted.

    Supported clients:
        - anthropic.Anthropic (messages.create)
        - openai.OpenAI (chat.completions.create)
    """
    client_type = type(client).__name__

    if client_type == "Anthropic":
        _wrap_anthropic(client)
    elif client_type == "OpenAI":
        _wrap_openai(client)
    elif client_type == "module" and hasattr(client, "completion"):
        # LiteLLM module: litellm.completion()
        _wrap_litellm(client)
    else:
        raise ValueError(
            f"unsupported client type: {client_type}. "
            "Supported: anthropic.Anthropic, openai.OpenAI, litellm module"
        )

    return client


def _serialize_request(kwargs: dict) -> bytes:
    """Serialize API call kwargs to stable bytes for hashing."""
    # Sort keys for deterministic serialization
    return json.dumps(kwargs, sort_keys=True, default=str).encode()


def _wrap_anthropic(client):
    """Monkey-patch anthropic.Anthropic.messages.create."""
    original_create = client.messages.create

    @wraps(original_create)
    def patched_create(*args, **kwargs):
        # Replay mode: serve from cache
        if _active_replayer is not None:
            req_bytes = _serialize_request(kwargs)
            cached = _active_replayer.lookup(req_bytes)
            if cached is not None:
                # Reconstruct the response object
                data = json.loads(cached)
                return _reconstruct_anthropic_response(data)
            # Fall through to real API if miss (or raise)
            raise LookupError(
                f"no cached response for request hash "
                f"{hashlib.sha256(req_bytes).hexdigest()[:16]}"
            )

        # Record mode: call real API and capture
        if _active_recorder is not None:
            req_bytes = _serialize_request(kwargs)
            t0 = time.monotonic()
            response = original_create(*args, **kwargs)
            latency_ms = int((time.monotonic() - t0) * 1000)

            # Serialize response
            resp_bytes = json.dumps(response.model_dump(), default=str).encode()
            _active_recorder.capture(req_bytes, resp_bytes, latency_ms)
            return response

        # No active session — pass through
        return original_create(*args, **kwargs)

    client.messages.create = patched_create


def _wrap_openai(client):
    """Monkey-patch openai.OpenAI.chat.completions.create."""
    original_create = client.chat.completions.create

    @wraps(original_create)
    def patched_create(*args, **kwargs):
        if _active_replayer is not None:
            req_bytes = _serialize_request(kwargs)
            cached = _active_replayer.lookup(req_bytes)
            if cached is not None:
                data = json.loads(cached)
                return _reconstruct_openai_response(data)
            raise LookupError(
                f"no cached response for request hash "
                f"{hashlib.sha256(req_bytes).hexdigest()[:16]}"
            )

        if _active_recorder is not None:
            req_bytes = _serialize_request(kwargs)
            t0 = time.monotonic()
            response = original_create(*args, **kwargs)
            latency_ms = int((time.monotonic() - t0) * 1000)
            resp_bytes = json.dumps(response.model_dump(), default=str).encode()
            _active_recorder.capture(req_bytes, resp_bytes, latency_ms)
            return response

        return original_create(*args, **kwargs)

    client.chat.completions.create = patched_create


def _wrap_litellm(module):
    """Monkey-patch litellm.completion to intercept calls."""
    original_completion = module.completion

    @wraps(original_completion)
    def patched_completion(*args, **kwargs):
        if _active_replayer is not None:
            req_bytes = _serialize_request(kwargs)
            cached = _active_replayer.lookup(req_bytes)
            if cached is not None:
                data = json.loads(cached)
                return _reconstruct_openai_response(data)
            raise LookupError(
                f"no cached response for request hash "
                f"{hashlib.sha256(req_bytes).hexdigest()[:16]}"
            )

        if _active_recorder is not None:
            req_bytes = _serialize_request(kwargs)
            t0 = time.monotonic()
            response = original_completion(*args, **kwargs)
            latency_ms = int((time.monotonic() - t0) * 1000)
            resp_bytes = json.dumps(response.model_dump(), default=str).encode()
            _active_recorder.capture(req_bytes, resp_bytes, latency_ms)
            return response

        return original_completion(*args, **kwargs)

    module.completion = patched_completion


def _reconstruct_anthropic_response(data: dict):
    """Reconstruct an Anthropic Message from cached dict."""
    try:
        from anthropic.types import Message
        return Message(**data)
    except Exception:
        # Fallback: return raw dict wrapped in a simple namespace
        return _DictNamespace(data)


def _reconstruct_openai_response(data: dict):
    """Reconstruct an OpenAI ChatCompletion from cached dict."""
    try:
        from openai.types.chat import ChatCompletion
        return ChatCompletion(**data)
    except Exception:
        return _DictNamespace(data)


class _DictNamespace:
    """Simple namespace that allows attribute access on a dict."""

    def __init__(self, data: dict):
        self._data = data
        for k, v in data.items():
            if isinstance(v, dict):
                setattr(self, k, _DictNamespace(v))
            elif isinstance(v, list):
                setattr(self, k, [
                    _DictNamespace(i) if isinstance(i, dict) else i for i in v
                ])
            else:
                setattr(self, k, v)

    def __repr__(self):
        return f"DictNamespace({self._data})"

    def model_dump(self):
        return self._data
