import importlib.util
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("quant_config", Path(__file__).with_name("quant_public_https_config.py"))
config = importlib.util.module_from_spec(spec)
spec.loader.exec_module(config)


class PublicHttpsConfigTests(unittest.TestCase):
    def test_requires_tls_and_rejects_ambiguous_flags(self):
        for text in ("", "ACCOUNT_HTTPS_ENABLED=false\n", "ACCOUNT_HTTPS_ENABLED=true\nACCOUNT_HTTPS_ENABLED=true\n",
                     "ACCOUNT_HTTPS_ENABLED=true\nQUANT_PUBLIC_HTTPS_ENABLED=1\n",
                     'ACCOUNT_HTTPS_ENABLED=\'true"\n'):
            with self.assertRaises(ValueError):
                config.inspect_text(text)
        self.assertEqual("absent", config.inspect_text('ACCOUNT_HTTPS_ENABLED="true"\n'))

    def test_updates_only_one_flag_and_preserves_unrelated_lines(self):
        original = "# kept\nACCOUNT_HTTPS_ENABLED=true\nUNRELATED=fixture\n"
        enabled = config.update_text(original, "absent", "true")
        self.assertEqual(original + "QUANT_PUBLIC_HTTPS_ENABLED=true\n", enabled)
        self.assertEqual(original, config.update_text(enabled, "true", "absent"))
        with self.assertRaises(ValueError):
            config.update_text(enabled, "false", "absent")

    def test_rollback_preserves_intervening_unrelated_changes(self):
        changed = "ACCOUNT_HTTPS_ENABLED=true\nQUANT_PUBLIC_HTTPS_ENABLED=true\nNEW=kept\n"
        self.assertEqual("ACCOUNT_HTTPS_ENABLED=true\nNEW=kept\n", config.update_text(changed, "true", "absent"))
        with self.assertRaises(ValueError):
            config.update_text(changed.replace("ENABLED=true\nNEW", "ENABLED=false\nNEW"), "true", "absent")

    def test_atomic_file_update_and_failure_keep_original(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "env"
            path.write_text("ACCOUNT_HTTPS_ENABLED=true\nUNRELATED=fixture\n", encoding="utf-8")
            config.update_file(path, "absent", "true")
            saved = path.read_bytes()
            with self.assertRaises(ValueError):
                config.update_file(path, "false", "absent")
            self.assertEqual(saved, path.read_bytes())
            self.assertEqual(["env"], [item.name for item in Path(directory).iterdir()])
            config.update_file(path, "true", "absent")
            self.assertNotIn(config.KEY, path.read_text())


if __name__ == "__main__":
    unittest.main()
