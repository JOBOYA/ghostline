"""Replay cached responses from a .ghostline file."""

import hashlib
from pathlib import Path

from ghostline.format import GhostlineReader


class GhostlineReplayer:
    """Serves cached responses by matching request hashes.

    Usage:
        replayer = GhostlineReplayer("run.ghostline")
        replayer.start()
        # ... make API calls via wrapped client — served from cache ...
        replayer.stop()
    """

    def __init__(self, path: str | Path):
        self.path = Path(path)
        self._file = None
        self._reader = None
        self._cache: dict[bytes, bytes] = {}
        self._started = False
        self.hits = 0
        self.misses = 0

    def start(self):
        if self._started:
            return
        self._file = open(self.path, "rb")
        self._reader = GhostlineReader(self._file)
        # Pre-load all frames into a hash map
        for frame in self._reader:
            self._cache[frame.request_hash] = frame.response_bytes
        self._started = True

    def stop(self):
        if not self._started:
            return
        self._file.close()
        self._file = None
        self._reader = None
        self._cache.clear()
        self._started = False

    def lookup(self, request_bytes: bytes) -> bytes | None:
        """Look up a cached response by request body hash."""
        if not self._started:
            raise RuntimeError("replayer not started")
        req_hash = hashlib.sha256(request_bytes).digest()
        result = self._cache.get(req_hash)
        if result is not None:
            self.hits += 1
        else:
            self.misses += 1
        return result

    def __enter__(self):
        self.start()
        return self

    def __exit__(self, *exc):
        self.stop()
