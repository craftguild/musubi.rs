#!/usr/bin/env python3
"""Render the sole current manifest from reviewed corpus records on standard input.

This repository-only producer does not read an earlier manifest or modify artifact evidence.
Callers own private staging and transactional installation of the emitted bytes.
"""

import json
import re
import sys


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("duplicate inventory field")
        result[key] = value
    return result


def render(raw):
    if len(raw) > 1024 * 1024:
        raise ValueError("inventory exceeds its bound")
    document = json.loads(raw, object_pairs_hook=unique_object)
    if (not isinstance(document, dict)
        or set(document) != {"schema_version", "contracts"}
        or document["schema_version"] != "musubi/abi-corpus-inventory-v1"
        or not isinstance(document["contracts"], list)):
        raise ValueError("invalid reviewed corpus inventory")
    for contract in document["contracts"]:
        if (not isinstance(contract, dict) or set(contract) != {"id", "abi_sha256"}
            or not isinstance(contract["abi_sha256"], str)
            or not re.fullmatch(r"0x[a-f0-9]{64}", contract["abi_sha256"])):
            raise ValueError("invalid corpus input digest")
    ids = [contract.get("id") for contract in document["contracts"]]
    if (
        len(ids) != 25
        or any(not isinstance(value, str) or not re.fullmatch(r"[a-z][a-z0-9_]{0,255}", value) for value in ids)
        or len(set(ids)) != len(ids)
    ):
        raise ValueError("invalid corpus identifier inventory")
    source = (
        'schema_version = "musubi/config-v4"\n\n'
        '[runtime]\nsource = "path"\nroot = "../../.."\n\n'
        '[package]\nname = "musubi-abi-corpus"\n'
        'version = "0.1.0"\nedition = "2024"\n\n'
        '[generation]\noutput = "sdk"\n'
    )
    for identifier in ids:
        source += (
            f'\n[[contracts]]\nid = "{identifier}"\nrust_module = "{identifier}"\n'
            f'kind = "interface"\nabi = "abi/{identifier}.json"\n'
        )
    return source


if __name__ == "__main__":
    try:
        sys.stdout.write(render(sys.stdin.buffer.read(1024 * 1024 + 1)))
    except (ValueError, TypeError, AttributeError, UnicodeError, RecursionError):
        print("invalid corpus manifest authority", file=sys.stderr)
        sys.exit(1)
