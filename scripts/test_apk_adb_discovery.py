import importlib.util
from pathlib import Path
import subprocess
import unittest

spec = importlib.util.spec_from_file_location('discovery', Path(__file__).with_name('apk_adb_discovery.py'))
discovery = importlib.util.module_from_spec(spec)
spec.loader.exec_module(discovery)


class DiscoveryTest(unittest.TestCase):
    def fixture(self, mode):
        self.calls = []

        def invoke(args, timeout):
            self.calls.append(args)
            code, text = 0, ''
            if args[1:3] == ['devices', '-l']:
                text = 'List of devices attached\nusb-a device\nemulator-5554 device'
            elif args[1:3] == ['mdns', 'services']:
                text = 'adb-hw-a-session _adb-tls-connect._tcp 192.168.1.10:37001'
            elif args[1] == 'connect':
                pass
            elif args[3] == 'get-state':
                if mode == 'offline':
                    code, text = 1, 'offline'
                elif mode == 'unauthorized':
                    code, text = 1, 'unauthorized'
                elif mode == 'mdns':
                    code, text = (0, 'device') if args[2].endswith(':37001') else (1, 'offline')
                elif (mode == 'usb' and args[2] == 'usb-a') or (mode != 'usb' and ':' in args[2]):
                    text = 'device'
                else:
                    code = 1
            elif args[3:5] == ['shell', 'getprop']:
                text = 'other' if mode == 'mismatch' else 'hw-a'
            else:
                raise AssertionError(args)
            return subprocess.CompletedProcess(args, code, text, '')

        return invoke

    def test_usb_priority(self):
        self.assertEqual(discovery.resolve('adb', 'hw-a', '192.168.1.1:5555', self.fixture('usb')), ('online', 'usb-a'))
        self.assertFalse(any('connect' in args or 'emulator-5554' in args for args in self.calls))

    def test_wireless_and_mdns(self):
        for mode, endpoint in [('wifi', '192.168.1.1:5555'), ('mdns', '192.168.1.10:37001')]:
            self.assertEqual(discovery.resolve('adb', 'hw-a', '192.168.1.1:5555', self.fixture(mode)), ('online', endpoint))

    def test_offline_and_unauthorized_and_reused_ip(self):
        for mode, status in [('offline', 'offline'), ('unauthorized', 'unauthorized'), ('mismatch', 'identity_mismatch')]:
            self.assertEqual(discovery.resolve('adb', 'hw-a', '192.168.1.1:5555', self.fixture(mode)), (status, ''))

    def test_invalid_identity(self):
        with self.assertRaises(ValueError):
            discovery.resolve('adb', 'bad;identity', '', self.fixture('usb'))


if __name__ == '__main__':
    unittest.main()
