"""Context managers for recording and replaying."""

from contextlib import contextmanager
from pathlib import Path

from ghostline.recorder import GhostlineRecorder
from ghostline.replayer import GhostlineReplayer
from ghostline.wrapper import set_recorder, set_replayer


@contextmanager
def record(path: str | Path):
    """Record all wrapped API calls to a .ghostline file.

    Usage:
        client = ghostline.wrap(Anthropic())
        with ghostline.record("run.ghostline"):
            response = client.messages.create(...)
    """
    recorder = GhostlineRecorder(path)
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
