"""Record LLM API calls to a .ghostline file."""

import hashlib
import json
import time
from pathlib import Path

from ghostline.format import Frame, GhostlineWriter


class GhostlineRecorder:
    """Records API calls by intercepting client methods.

    Usage:
        recorder = GhostlineRecorder("run.ghostline")
        recorder.start()
        # ... make API calls via wrapped client ...
        recorder.stop()
    """

    def __init__(self, path: str | Path):
        self.path = Path(path)
        self._file = None
        self._writer = None
        self._started = False

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
        """Record a single request/response pair."""
        if not self._started:
            raise RuntimeError("recorder not started")
        timestamp = int(time.time() * 1000)
        frame = Frame(request_bytes, response_bytes, latency_ms, timestamp)
        self._writer.append(frame)

    def __enter__(self):
        self.start()
        return self

    def __exit__(self, *exc):
        self.stop()
