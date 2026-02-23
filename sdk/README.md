# ghostline Python SDK

> **Not yet published.** Coming soon.

```python
pip install ghostline  # soon
```

## Usage

```python
import ghostline
from anthropic import Anthropic

client = ghostline.wrap(Anthropic())

# Record
with ghostline.record("run.ghostline"):
    response = client.messages.create(...)

# Replay (zero API calls)
with ghostline.replay("run.ghostline"):
    response = client.messages.create(...)  # served from file
```

## Supported providers

- [ ] Anthropic
- [ ] OpenAI
- [ ] LiteLLM (any provider via proxy)
