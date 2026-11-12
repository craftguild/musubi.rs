#!/usr/bin/env python3
"""Real-CLI regressions for raw ABI caps, canonical expansion, and generated output budgets."""

import json
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent.parent
ABI_CAP = 8 * 1024 * 1024


def fixture(root, events, limits=""):
    fields = [{"name": f"field_{index}_" + "x" * 64, "type": "uint256"}
              for index in range(16)]
    abi = [{"type": "event", "name": f"E{index}", "inputs": fields}
           for index in range(events)]
    raw = json.dumps(abi, separators=(",", ":")).encode()
    (root / "abi.json").write_bytes(raw)
    (root / "musubi.toml").write_text(
        'schema_version = "musubi/config-v4"\n'
        '[runtime]\nsource = "path"\nroot = "../.."\n'
        '[package]\nname = "canonical-budget-sdk"\nversion = "0.1.0"\nedition = "2024"\n'
        '[generation]\noutput = "sdk"\n'
        '[[contracts]]\nid = "many_events"\nkind = "interface"\nabi = "abi.json"\n'
        + limits)
    return raw


def cli(root, operation, succeeds=True, code=None):
    output = subprocess.run(
        ["cargo", "run", "--quiet", "--locked", "-p", "musubi-cli", "--",
         "--format", "json", "--manifest", str(root / "musubi.toml"), operation],
        cwd=ROOT, capture_output=True, text=True, timeout=360)
    assert (output.returncode == 0) == succeeds, output.stderr
    if not succeeds:
        errors = [item for item in json.loads(output.stderr)["diagnostics"]
                  if item["severity"] == "error"]
        assert errors, output.stderr
        if code is not None:
            assert errors[0]["code"] == code, output.stderr
        assert not (root / "sdk").exists()
    print(f"canonical ABI budget: {root.name} / {operation} / expected result", flush=True)


def main():
    (ROOT / "target").mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="canonical-abi-", dir=ROOT / "target") as temporary:
        root = Path(temporary)
        # Each input lives exactly two levels below ROOT so its Path runtime is explicit.
        raw = fixture(root, 4096)
        assert len(raw) == 6_818_731 and len(raw) <= ABI_CAP
        for operation in ["generate", "check", "doctor"]:
            cli(root, operation)
        canonical = root / "sdk/assets/many_events/abi.json"
        assert canonical.stat().st_size == 9_165_741 > ABI_CAP
        print("large positive: 4096 events / 65536 fields / canonical 9165741 bytes", flush=True)

    with tempfile.TemporaryDirectory(prefix="canonical-lowered-", dir=ROOT / "target") as temporary:
        root = Path(temporary)
        raw = fixture(root, 1)
        fixture(root, 1, f"[limits]\nabi_bytes = {len(raw)}\n")
        for operation in ["generate", "check", "doctor", "update", "check"]:
            cli(root, operation)
        canonical_bytes = (root / "sdk/assets/many_events/abi.json").stat().st_size
        assert canonical_bytes > len(raw)

    cases = [(f"abi_bytes = {len(raw) - 1}", "MSB-E003"),
             (f"generated_bytes = {canonical_bytes - 1}", "MSB-E003"),
             (f"generated_bytes = {canonical_bytes}", None)]
    for limits, code in cases:
        with tempfile.TemporaryDirectory(prefix="canonical-negative-", dir=ROOT / "target") as temporary:
            root = Path(temporary)
            fixture(root, 1, f"[limits]\n{limits}\n")
            cli(root, "generate", succeeds=False, code=code)


if __name__ == "__main__":
    main()
