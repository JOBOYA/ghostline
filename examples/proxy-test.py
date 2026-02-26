#!/usr/bin/env python3
"""Test the ghostline proxy — run after starting: ghostline proxy --out /tmp/test-runs/"""
import os, anthropic

# Point to local proxy
client = anthropic.Anthropic(
    base_url=os.environ.get("ANTHROPIC_BASE_URL", "http://localhost:9000")
)

msg = client.messages.create(
    model="claude-3-haiku-20240307",
    max_tokens=50,
    messages=[{"role": "user", "content": "Say 'proxy test successful' in exactly 3 words"}]
)
print("Response:", msg.content[0].text)
print("Test passed! Check /tmp/test-runs/ for the .ghostline file")
