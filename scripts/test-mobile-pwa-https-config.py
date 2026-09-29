"""Offline acceptance for the single-flag production configuration transaction."""
import tempfile
import unittest
from pathlib import Path

import mobile_pwa_https_config as config


class PwaHttpsConfigTests(unittest.TestCase):
    BASE = "# keep this comment\nACCOUNT_HTTPS_ENABLED=true\nPUBLIC_URL=http://example.test:8080\nOTHER=preserved\n"

    def test_enable_disable_and_rollback_preserve_other_settings(self):
        enabled = config.update_text(self.BASE, "absent", "true")
        self.assertEqual(config.inspect_text(enabled), "true")
        self.assertTrue(enabled.startswith(self.BASE))
        disabled = config.update_text(enabled, "true", "false")
        self.assertEqual(config.inspect_text(disabled), "false")
        self.assertEqual(config.update_text(disabled, "false", "absent"), self.BASE)

    def test_rejects_ambiguous_tls_and_concurrent_flag_changes(self):
        for text in [self.BASE.replace("ENABLED=true", "ENABLED=false"),
                     self.BASE + "MOBILE_PWA_HTTPS_ENABLED=maybe\n",
                     self.BASE + "MOBILE_PWA_HTTPS_ENABLED=true\nMOBILE_PWA_HTTPS_ENABLED=false\n"]:
            with self.assertRaises(ValueError):
                config.update_text(text, "absent", "true")
        with self.assertRaises(ValueError):
            config.update_text(self.BASE, "false", "true")

    def test_atomic_file_update_and_failed_compare_preserve_bytes(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "server.env"
            path.write_text(self.BASE, encoding="utf-8")
            config.update_file(path, "absent", "true")
            before = path.read_bytes()
            with self.assertRaises(ValueError):
                config.update_file(path, "absent", "false")
            self.assertEqual(path.read_bytes(), before)
            config.update_file(path, "true", "absent")
            self.assertEqual(path.read_text(encoding="utf-8"), self.BASE)
            self.assertEqual(list(Path(directory).iterdir()), [path])


if __name__ == "__main__":
    unittest.main()
