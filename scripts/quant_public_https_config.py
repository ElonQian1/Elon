#!/usr/bin/env python3
"""Atomic compare-and-set for one public TLS flag; never print unrelated dotenv data."""
import os
from pathlib import Path
import stat
import sys
import tempfile

KEY = "QUANT_PUBLIC_HTTPS_ENABLED"
VALUES = {"absent", "true", "false"}


def inspect_text(text):
    seen = {}
    for line in text.splitlines():
        key, separator, value = line.partition("=")
        if separator and key.strip() in (KEY, "ACCOUNT_HTTPS_ENABLED"):
            key = key.strip()
            if key in seen:
                raise ValueError("duplicate_tls_setting")
            value = value.strip()
            if value not in ("true", "false", '"true"', '"false"', "'true'", "'false'"):
                raise ValueError("invalid_tls_setting")
            value = value.strip("\"'")
            seen[key] = value
    if seen.get("ACCOUNT_HTTPS_ENABLED") != "true":
        raise ValueError("account_tls_required")
    return seen.get(KEY, "absent")


def update_text(text, expected, desired):
    if expected not in VALUES or desired not in VALUES:
        raise ValueError("invalid_flag_value")
    if inspect_text(text) != expected:
        raise ValueError("flag_changed")
    lines = [line for line in text.splitlines(keepends=True)
             if not ("=" in line and line.partition("=")[0].strip() == KEY)]
    result = "".join(lines)
    if desired != "absent":
        if result and not result.endswith("\n"):
            result += "\n"
        result += f"{KEY}={desired}\n"
    return result


def update_file(path, expected, desired):
    path = Path(path)
    if path.is_symlink() or not path.is_file():
        raise ValueError("regular_environment_file_required")
    before = path.stat()
    text = path.read_text(encoding="utf-8")
    updated = update_text(text, expected, desired)
    fd, temporary = tempfile.mkstemp(prefix=".quant-public-https-", dir=path.parent)
    try:
        with os.fdopen(fd, "w", encoding="utf-8", newline="") as stream:
            stream.write(updated)
            stream.flush()
            os.fsync(stream.fileno())
        os.chmod(temporary, stat.S_IMODE(before.st_mode))
        if hasattr(os, "chown"):
            os.chown(temporary, before.st_uid, before.st_gid)
        # Do not overwrite concurrent edits made outside the shared deploy lock.
        if path.is_symlink() or path.stat().st_ino != before.st_ino or path.read_text(encoding="utf-8") != text:
            raise ValueError("environment_changed")
        os.replace(temporary, path)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)


def main(args):
    try:
        if len(args) == 2 and args[0] == "inspect":
            print(inspect_text(Path(args[1]).read_text(encoding="utf-8")))
        elif len(args) == 4 and args[0] == "set":
            update_file(args[1], args[2], args[3])
        else:
            raise ValueError("invalid_arguments")
    except (ValueError, OSError, UnicodeError):
        print("QUANT_HTTPS_CONFIG_ERROR=configuration_rejected", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
