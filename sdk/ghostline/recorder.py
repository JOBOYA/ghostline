"""Record LLM API calls to a .ghostline file."""

import hashlib
import json
import time
from pathlib import Path

from ghostline.format import Frame, GhostlineWriter
from ghostline.scrub import ScrubConfig, scrub_bytes


class GhostlineRecorder:
    """Records API calls by intercepting client methods.

    Usage:
        recorder = GhostlineRecorder("run.ghostline")
        recorder.start()
        # ... make API calls via wrapped client ...
        recorder.stop()

    Scrubbing:
        recorder = GhostlineRecorder("run.ghostline", scrub=True)
        # API keys, emails, tokens are automatically redacted before writing

        # Custom scrub config:
        from ghostline.scrub import ScrubConfig
        config = ScrubConfig(custom_strings=[("my-secret", "[REDACTED]")])
        recorder = GhostlineRecorder("run.ghostline", scrub=config)
    """

    def __init__(self, path: str | Path, scrub: bool | ScrubConfig = False):
        self.path = Path(path)
        self._file = None
        self._writer = None
        self._started = False
        if isinstance(scrub, ScrubConfig):
            self._scrub_config = scrub
        elif scrub:
            self._scrub_config = ScrubConfig()
        else:
            self._scrub_config = None

    def start(self):
        if self._started:
            return
        self._file = open(self.path, "wb")
        started_at = int(time.time() * 1000)
        self._writer = GhostlineWriter(self._file, started_at)
        self._started = True

    def stop(self):
        if not self._started:
            return
        self._writer.finish()
        self._file.close()
        self._file = None
        self._writer = None
        self._started = False

    def capture(self, request_bytes: bytes, response_bytes: bytes, latency_ms: int):
        """Record a single request/response pair.

        If scrubbing is enabled, sensitive data is redacted before the
        frame is written to disk. The hash is computed on scrubbed data.
        """
        if not self._started:
            raise RuntimeError("recorder not started")
        if self._scrub_config is not None:
            request_bytes = scrub_bytes(request_bytes, self._scrub_config)
            response_bytes = scrub_bytes(response_bytes, self._scrub_config)
        timestamp = int(time.time() * 1000)
        frame = Frame(request_bytes, response_bytes, latency_ms, timestamp)
        self._writer.append(frame)

    def __enter__(self):
        self.start()
        return self

    def __exit__(self, *exc):
        self.stop()
