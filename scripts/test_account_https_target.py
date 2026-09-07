#!/usr/bin/env python3
"""Offline contract tests for deployment target parsing and CLI output."""

import contextlib
import io
import json
import unittest
from unittest import mock

from account_https_target import TargetError, main, parse_target


CERT = "/etc/account-tls/fullchain.pem"
KEY = "/etc/account-tls/privkey.pem"
FIELDS = {
    "origin", "host", "port", "listen_addr", "certificate_path",
    "private_key_path", "certificate_source", "challenge", "validation_port",
}


class TargetTests(unittest.TestCase):
    def test_legacy_public_ipv4_is_managed_and_exact(self):
        self.assertEqual(parse_target("43.139.149.158"), {
            "origin": "https://43.139.149.158",
            "host": "43.139.149.158", "port": 443,
            "listen_addr": "0.0.0.0:443",
            "certificate_path": "/etc/letsencrypt/live/43.139.149.158/fullchain.pem",
            "private_key_path": "/etc/letsencrypt/live/43.139.149.158/privkey.pem",
            "certificate_source": "managed_ip", "challenge": "http-01",
            "validation_port": 80,
        })

    def test_https_ipv4_port_and_trailing_slash(self):
        for suffix in ("", "/", ":443", ":443/"):
            with self.subTest(suffix=suffix):
                self.assertEqual(parse_target("https://43.139.149.158" + suffix),
                                 parse_target("43.139.149.158"))
        for port in (1, 8443, 65535):
            with self.subTest(port=port):
                plan = parse_target(f"https://43.139.149.158:{port}/")
                self.assertEqual(plan["port"], port)
                self.assertEqual(plan["origin"], f"https://43.139.149.158:{port}")
                self.assertEqual(plan["listen_addr"], f"0.0.0.0:{port}")
                self.assertEqual(plan["validation_port"], 80)

    def test_external_dns_and_ipv4_paths_are_not_opened(self):
        with mock.patch("builtins.open", side_effect=AssertionError("file access")):
            for target in ("https://API.Example.com:8443/", "https://43.139.149.158:8443"):
                with self.subTest(target=target):
                    plan = parse_target(target, CERT, KEY)
                    self.assertEqual(set(plan), FIELDS)
                    self.assertEqual(plan["origin"], target.lower().rstrip("/"))
                    self.assertEqual(plan["certificate_path"], CERT)
                    self.assertEqual(plan["private_key_path"], KEY)
                    self.assertEqual(plan["certificate_source"], "external")
                    self.assertEqual(plan["challenge"], "external")
                    self.assertIsNone(plan["validation_port"])

    def test_dns_labels_and_name_limits(self):
        for host in ("example.com", "a.example.com", "xn--bcher-kva.example.com",
                     "a" * 63 + ".example.com"):
            self.assertEqual(parse_target("https://" + host, CERT, KEY)["host"], host)
        for host in ("localhost", "api.localhost", "example", "-api.example.com",
                     "api-.example.com", "api_name.example.com", ".example.com",
                     "api..example.com", "example.com.", "a" * 64 + ".example.com",
                     ".".join(["a" * 63] * 4), "example.123", "例子.example.com"):
            with self.subTest(host=host), self.assertRaises(TargetError):
                parse_target("https://" + host, CERT, KEY)

    def test_private_and_non_ipv4_addresses_are_rejected_even_with_certificates(self):
        for host in ("127.0.0.1", "10.0.0.1", "172.16.0.1", "192.168.1.1",
                     "169.254.1.1", "100.64.0.1", "0.0.0.0", "192.0.2.1",
                     "198.51.100.1", "203.0.113.1", "224.0.0.1", "240.0.0.1",
                     "255.255.255.255", "999.1.2.3", "043.139.149.158",
                     "[2606:4700:4700::1111]", "[::1]"):
            with self.subTest(host=host), self.assertRaises(TargetError):
                parse_target("https://" + host, CERT, KEY)

    def test_invalid_origin_and_port_are_rejected(self):
        for target in ("", "example.com", "43.139.149.158:8443", "http://example.com",
                       "HTTPS://example.com", "https://user@example.com",
                       "https://user:pass@example.com", "https://example.com/path",
                       "https://example.com//", "https://example.com?",
                       "https://example.com#", "https://example.com/?x=1",
                       "https://example.com/#x", "https://example.com:0",
                       "https://example.com:65536", "https://example.com:-1",
                       "https://example.com:08443", "https://example.com:abc",
                       "https://example.com:", "https://example.com:8443:443",
                       "https://example%2ecom", "https://example.com\\evil"):
            with self.subTest(target=target), self.assertRaises(TargetError):
                parse_target(target, CERT, KEY)

    def test_control_and_whitespace_are_rejected_before_url_parsing(self):
        for char in (" ", "\t", "\n", "\r", "\x00", "\x7f", "\x85", "\u2003"):
            for target in (char + "https://example.com", "https://example.com" + char,
                           "https://exam" + char + "ple.com"):
                with self.subTest(target=repr(target)), self.assertRaises(TargetError):
                    parse_target(target, CERT, KEY)
            with self.subTest(path=repr(char)), self.assertRaises(TargetError):
                parse_target("https://example.com", "/etc/" + char + "cert.pem", KEY)

    def test_dns_requires_pair_and_paths_must_be_distinct_absolute_posix(self):
        for target, cert, key in (
            ("https://example.com", "", ""), ("43.139.149.158", CERT, ""),
            ("43.139.149.158", "", KEY), ("https://example.com", "cert.pem", KEY),
            ("https://example.com", CERT, "key.pem"),
            ("https://example.com", "C:\\cert.pem", KEY),
            ("https://example.com", "/", KEY), ("https://example.com", CERT, CERT),
            ("https://example.com", CERT, "/etc/account-tls/./fullchain.pem"),
            ("https://example.com", CERT, "/etc/account-tls/x/../fullchain.pem"),
        ):
            with self.subTest(target=target, cert=cert, key=key), self.assertRaises(TargetError):
                parse_target(target, cert, key)

    def test_invalid_argument_types_use_fixed_errors(self):
        for args in ((None,), (12,), ("43.139.149.158", None, KEY),
                     ("43.139.149.158", CERT, [])):
            with self.subTest(args=args), self.assertRaises(TargetError):
                parse_target(*args)

    def test_certificate_paths_reject_dotenv_expansion_escaping_and_non_ascii(self):
        for fragment in (
            "$HOME", "${HOME}", "'quoted'", '"quoted"', "\\escaped", "#comment",
            "`command`", "key=value", "C:", "two words", "\u8bc1\u4e66", "\nnext",
            "\ttab", "\x00null", "semi;colon", "(paren)", "[bracket]", "!", "%",
            "&", "?", "+", "@", "*", "~", "|", "<", ">", ",",
        ):
            unsafe_path = "/etc/account-tls/" + fragment + "/material.pem"
            for cert, key in ((unsafe_path, KEY), (CERT, unsafe_path)):
                with self.subTest(fragment=repr(fragment), cert_side=cert == unsafe_path):
                    with self.assertRaises(TargetError) as raised:
                        parse_target("https://example.com", cert, key)
                    self.assertIn(str(raised.exception), {
                        "invalid_certificate_paths", "absolute_posix_certificate_paths_required",
                    })


class CliTests(unittest.TestCase):
    def invoke(self, args):
        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            try:
                code = main(args)
            except SystemExit as error:
                code = error.code
        return code, out.getvalue(), err.getvalue()

    def test_json_has_only_fixed_fields_and_no_mutation(self):
        code, out, err = self.invoke(["https://43.139.149.158:8443"])
        self.assertEqual((code, err), (0, ""))
        result = json.loads(out)
        self.assertEqual(set(result), FIELDS | {
            "deployment_mutated", "certificate_issuance_performed",
        })
        self.assertIs(result["deployment_mutated"], False)
        self.assertIs(result["certificate_issuance_performed"], False)
        self.assertIsInstance(result["port"], int)

    def test_lines_have_exact_order_no_shell_syntax_and_seven_lines(self):
        code, out, err = self.invoke([
            "https://example.com:8443/", "--certificate-path", CERT,
            "--private-key-path", KEY, "--format", "lines",
        ])
        self.assertEqual((code, err), (0, ""))
        self.assertEqual(out.splitlines(), [
            "https://example.com:8443", "example.com", "8443", "0.0.0.0:8443",
            CERT, KEY, "external",
        ])

    def test_invalid_plan_prints_no_plan_or_input(self):
        code, out, err = self.invoke(["https://private-canary@example.com"])
        self.assertEqual((code, out), (2, ""))
        self.assertEqual(err, "ACCOUNT_HTTPS_TARGET_ERROR=invalid_https_origin\n")
        self.assertNotIn("private-canary", err)

    def test_cli_missing_arguments_unknown_options_and_format_fail(self):
        for args in ([], ["43.139.149.158", "--format", "shell"],
                     ["43.139.149.158", "--certificate-path"],
                     ["43.139.149.158", "--unknown"]):
            with self.subTest(args=args):
                code, out, _ = self.invoke(args)
                self.assertEqual((code, out), (2, ""))


if __name__ == "__main__":
    unittest.main()
