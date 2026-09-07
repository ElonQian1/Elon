#!/usr/bin/env python3
"""Parse an account TLS target without accessing files, credentials or networks."""

import argparse
import ipaddress
import json
import posixpath
import re
import sys


class TargetError(ValueError):
    """A fixed error category, never an echo of supplied configuration."""


_HTTPS_TARGET = re.compile(
    r"https://(?P<host>[A-Za-z0-9.-]+)(?::(?P<port>[1-9][0-9]{0,4}))?/?"
)
_DNS_LABEL = re.compile(r"[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?")
# These paths are also written as bare dotenv values: no expansion or quoting syntax.
_CERTIFICATE_PATH = re.compile(r"/[A-Za-z0-9_./-]+")
_LINE_FIELDS = (
    "origin", "host", "port", "listen_addr", "certificate_path",
    "private_key_path", "certificate_source",
)


def _clean_text(value):
    return isinstance(value, str) and not any(
        char.isspace() or ord(char) < 32 or 127 <= ord(char) <= 159
        for char in value
    )


def _public_ipv4(host):
    try:
        address = ipaddress.IPv4Address(host)
    except ipaddress.AddressValueError:
        raise TargetError("public_ipv4_required") from None
    if not address.is_global or any((
        address.is_private, address.is_loopback, address.is_link_local,
        address.is_multicast, address.is_reserved, address.is_unspecified,
    )):
        raise TargetError("public_ipv4_required")
    return str(address)


def _host(host):
    if all(char in "0123456789." for char in host):
        return _public_ipv4(host), True
    host = host.lower()
    labels = host.split(".")
    if (
        len(host) > 253 or len(labels) < 2 or host.endswith(".localhost")
        or any(_DNS_LABEL.fullmatch(label) is None for label in labels)
        or labels[-1].isdigit()
    ):
        raise TargetError("public_dns_name_required")
    return host, False


def _external_paths(certificate_path, private_key_path):
    if not all(_clean_text(path) for path in (certificate_path, private_key_path)):
        raise TargetError("invalid_certificate_paths")
    if bool(certificate_path) != bool(private_key_path):
        raise TargetError("certificate_pair_required")
    if not certificate_path:
        return False
    if any(
        _CERTIFICATE_PATH.fullmatch(path) is None
        or posixpath.normpath(path) in ("/", "//")
        for path in (certificate_path, private_key_path)
    ):
        raise TargetError("absolute_posix_certificate_paths_required")
    if posixpath.normpath(certificate_path) == posixpath.normpath(private_key_path):
        raise TargetError("distinct_certificate_paths_required")
    return True


def parse_target(target, certificate_path="", private_key_path="") -> dict:
    """Return a fixed deployment plan; certificate paths are never opened."""
    if not _clean_text(target) or not target or not target.isascii():
        raise TargetError("invalid_target")
    if target.startswith("https://"):
        match = _HTTPS_TARGET.fullmatch(target)
        if match is None:
            raise TargetError("invalid_https_origin")
        host, is_ip = _host(match["host"])
        port = int(match["port"] or "443")
        if port > 65535:
            raise TargetError("invalid_https_port")
    else:
        host, is_ip, port = _public_ipv4(target), True, 443
    external = _external_paths(certificate_path, private_key_path)
    if not external:
        if not is_ip:
            raise TargetError("dns_certificate_pair_required")
        directory = "/etc/letsencrypt/live/" + host
        certificate_path = directory + "/fullchain.pem"
        private_key_path = directory + "/privkey.pem"
    authority = host if port == 443 else f"{host}:{port}"
    return {
        "origin": "https://" + authority,
        "host": host,
        "port": port,
        "listen_addr": f"0.0.0.0:{port}",
        "certificate_path": certificate_path,
        "private_key_path": private_key_path,
        "certificate_source": "external" if external else "managed_ip",
        "challenge": "external" if external else "http-01",
        "validation_port": None if external else 80,
    }


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("target")
    parser.add_argument("--certificate-path", default="")
    parser.add_argument("--private-key-path", default="")
    parser.add_argument("--format", choices=("json", "lines"), default="json")
    args = parser.parse_args(argv)
    try:
        plan = parse_target(args.target, args.certificate_path, args.private_key_path)
    except TargetError as error:
        print("ACCOUNT_HTTPS_TARGET_ERROR=" + str(error), file=sys.stderr)
        return 2
    if args.format == "lines":
        print("\n".join(str(plan[field]) for field in _LINE_FIELDS))
    else:
        print(json.dumps({
            **plan,
            "deployment_mutated": False,
            "certificate_issuance_performed": False,
        }, separators=(",", ":")))
    return 0


if __name__ == "__main__":
    sys.exit(main())
