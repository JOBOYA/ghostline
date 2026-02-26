"""Context managers for recording and replaying."""

from contextlib import contextmanager
from pathlib import Path

from ghostline.recorder import GhostlineRecorder
from ghostline.replayer import GhostlineReplayer
from ghostline.scrub import ScrubConfig
from ghostline.wrapper import set_recorder, set_replayer


@contextmanager
def record(path: str | Path, scrub: bool | ScrubConfig = True):
    """Record all wrapped API calls to a .ghostline file.

    Args:
        path: Output file path.
        scrub: Scrubbing is on by default. Pass False to disable, or a
            ScrubConfig for custom patterns.

    Usage:
        client = ghostline.wrap(Anthropic())
        with ghostline.record("run.ghostline"):
            response = client.messages.create(...)  # secrets auto-redacted
    """
    recorder = GhostlineRecorder(path, scrub=scrub)
    recorder.start()
    set_recorder(recorder)
    try:
        yield recorder
    finally:
        set_recorder(None)
        recorder.stop()


@contextmanager
def replay(path: str | Path):
    """Replay cached responses from a .ghostline file.

    Usage:
        client = ghostline.wrap(Anthropic())
        with ghostline.replay("run.ghostline"):
            response = client.messages.create(...)  # served from cache
    """
    replayer = GhostlineReplayer(path)
    replayer.start()
    set_replayer(replayer)
    try:
        yield replayer
    finally:
        set_replayer(None)
        replayer.stop()
