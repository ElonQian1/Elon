# 文件名：scripts/check-mobile-design-v2.py
"""只验证 V2 文档、路由和退役状态；不证明 APK、视觉或无障碍已验收。"""
from __future__ import annotations
import argparse
import hashlib
import json
from pathlib import Path, PurePosixPath
import re
import sys
import tempfile
import unittest

SPEC = "docs/design/mobile-design-system-v2.md"
POLICY = "docs/design/mobile-design-policy-v2.json"
EXPECTED = {
    "spec_version": "2.0",
    "system_ui_release_policy": "runtime-evidence-before-release",
    "implementation_route": "compose-material3-incremental",
    "cross_platform_policy": "semantic-parity-platform-adaptation",
}


def safe_path(root: Path, value: str) -> Path:
    """拒绝绝对路径、父目录跳转和逃逸根目录的符号链接。"""
    if not isinstance(value, str) or not value or "\\" in value:
        raise ValueError("invalid relative path")
    relative = PurePosixPath(value)
    if relative.is_absolute() or ".." in relative.parts or ":" in value:
        raise ValueError("unsafe relative path")
    target = (root / value).resolve()
    if not target.is_relative_to(root.resolve()):
        raise ValueError("path escapes repository")
    return target


def read_text(root: Path, path: str) -> str:
    # Windows CRLF 与 UTF-8 BOM 不应造成伪漂移；实质正文变化仍会失败。
    return safe_path(root, path).read_text(encoding="utf-8-sig").replace("\r\n", "\n")


def blob_sha(text: str) -> str:
    """使用 Git blob 身份对齐两仓规范；这不是签名或安全认证。"""
    data = text.encode("utf-8")
    return hashlib.sha1(b"blob " + str(len(data)).encode("ascii") + b"\0" + data).hexdigest()


def frontmatter(text: str) -> dict[str, str]:
    """仅解析本规范使用的标量 frontmatter，不把任意 YAML 当作可执行输入。"""
    if not text.startswith("---\n") or "\n---\n" not in text[4:]:
        return {}
    result = {}
    for line in text[4:].split("\n---\n", 1)[0].splitlines():
        key, sep, value = line.partition(":")
        if sep:
            result[key.strip()] = value.strip().strip("\"'")
    return result


def validate(root: Path, peer: Path | None = None) -> list[str]:
    errors: list[str] = []
    try:
        policy = json.loads(read_text(root, POLICY))
        if not isinstance(policy, dict):
            raise ValueError("policy must be an object")
        for key, expected in EXPECTED.items():
            if policy.get(key) != expected:
                errors.append("policy mismatch: " + key)
        text = read_text(root, SPEC)
        if frontmatter(text).get("spec_version") != "2.0":
            errors.append("canonical spec_version mismatch")
        digest = policy.get("canonical_blob_sha1", "")
        if not isinstance(digest, str) or not re.fullmatch(r"[0-9a-f]{40}", digest) or blob_sha(text) != digest:
            errors.append("canonical document digest mismatch")
        product = policy.get("product_document")
        if not isinstance(product, str) or not product:
            raise ValueError("product_document is required")
        if "mobile-design-system-v2.md" not in read_text(root, product):
            errors.append("product does not route to V2")
        sources = read_text(root, "docs/design/mobile-design-sources.md")
        for source_id in ("S01", "S02", "S07", "S08", "S09"):
            if source_id not in sources:
                errors.append("missing source: " + source_id)
        # 用明确入口白名单而不是声称做了全仓语义扫描。
        for entry in ("AGENTS.md", ".github/copilot-instructions.md", ".agents/skills/apk-ui/SKILL.md"):
            entry_text = read_text(root, entry)
            if SPEC not in entry_text:
                errors.append("missing V2 route: " + entry)
        contract = read_text(root, ".github/copilot-instructions.md")
        if "MOBILE_DESIGN_SYSTEM=2.0" not in contract or "APP_UI_SYSTEM_REFACTOR_POLICY=runtime-evidence-before-release" not in contract:
            errors.append("missing high-priority policy override")
        retired = policy.get("retired_documents")
        if not isinstance(retired, list) or not retired:
            raise ValueError("retired_documents must be a nonempty list")
        for path in retired:
            old = read_text(root, path)
            meta = frontmatter(old)
            if meta.get("version_status") != "superseded" or meta.get("superseded_by") != SPEC:
                errors.append("retired authority restored: " + path)
        if peer is not None and read_text(peer, SPEC) != text:
            errors.append("peer canonical document differs")
    except (OSError, UnicodeError, ValueError, TypeError) as exc:
        errors.append(type(exc).__name__ + ": " + str(exc))
    return errors


class GovernanceTests(unittest.TestCase):
    """合成目录回归；不访问网络、账户、Android 或原有工作区。"""
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.spec = '---\nspec_version: "2.0"\n---\n# 规范\n'
        self.policy = dict(EXPECTED, canonical_blob_sha1=blob_sha(self.spec), product_document="docs/Design.md", retired_documents=["docs/old.md"])
        self.write(SPEC, self.spec)
        self.write(POLICY, json.dumps(self.policy))
        self.write("docs/Design.md", "design/mobile-design-system-v2.md")
        self.write("docs/design/mobile-design-sources.md", "S01 S02 S07 S08 S09")
        for entry in ("AGENTS.md", ".agents/skills/apk-ui/SKILL.md"):
            self.write(entry, SPEC)
        self.write(".github/copilot-instructions.md", SPEC + "\nMOBILE_DESIGN_SYSTEM=2.0\nAPP_UI_SYSTEM_REFACTOR_POLICY=runtime-evidence-before-release")
        self.write("docs/old.md", "---\nversion_status: superseded\nsuperseded_by: " + SPEC + "\n---\n历史入口\n")

    def write(self, path: str, text: str) -> None:
        target = self.root / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(text, encoding="utf-8")

    def test_valid(self):
        self.assertEqual([], validate(self.root))

    def test_missing_route(self):
        self.write("AGENTS.md", "旧入口")
        self.assertTrue(validate(self.root))

    def test_missing_override(self):
        self.write(".github/copilot-instructions.md", SPEC)
        self.assertTrue(validate(self.root))

    def test_digest_drift(self):
        self.write(SPEC, self.spec + "未经登记的变化")
        self.assertTrue(validate(self.root))

    def test_retired_restored(self):
        self.write("docs/old.md", "---\nversion_status: current\n---\n旧规范")
        self.assertTrue(validate(self.root))

    def test_bad_json(self):
        self.write(POLICY, "{broken")
        self.assertTrue(validate(self.root))

    def test_unsafe_path(self):
        for path in ("../outside", "/tmp/outside", "C:/outside", "..\\outside"):
            with self.subTest(path=path), self.assertRaises(ValueError):
                safe_path(self.root, path)

    def test_bad_release_policy(self):
        self.policy["system_ui_release_policy"] = "publish-first"
        self.write(POLICY, json.dumps(self.policy))
        self.assertTrue(validate(self.root))

    def test_missing_file(self):
        (self.root / "docs/Design.md").unlink()
        self.assertTrue(validate(self.root))

    def test_crlf_and_bom(self):
        (self.root / SPEC).write_bytes(b"\xef\xbb\xbf" + self.spec.replace("\n", "\r\n").encode())
        self.assertEqual([], validate(self.root))

    def test_peer_drift(self):
        peer = self.root / "peer"
        (peer / SPEC).parent.mkdir(parents=True)
        (peer / SPEC).write_text(self.spec + "different", encoding="utf-8")
        self.assertTrue(validate(self.root, peer))

    def test_no_legacy_visual_lock(self):
        # 新颜色或 56dp 控件不应被治理检查阻挡；本检查不宣称它们视觉合格。
        self.write("android/example.txt", "button=56dp; color=#123456")
        self.assertEqual([], validate(self.root))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--peer-root", type=Path)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(GovernanceTests)
        return 0 if unittest.TextTestRunner(verbosity=2).run(suite).wasSuccessful() else 1
    errors = validate(args.root, args.peer_root)
    print(json.dumps({"status": "failed" if errors else "passed", "scope": "design-governance-only", "android_build": "not_run", "runtime_validation": "not_run", "errors": errors}, ensure_ascii=False, indent=2))
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
