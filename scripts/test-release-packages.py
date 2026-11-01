"""Behavioral package-verification checks against the actual local release artifacts."""
import importlib.util
from pathlib import Path
import tempfile
import shutil
import unittest
import zipfile

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('release_verify', ROOT/'scripts/verify-release-packages.py')
module = importlib.util.module_from_spec(spec);spec.loader.exec_module(module)


class ReleaseChecks(unittest.TestCase):
    def test_complete_current_release(self):
        result = module.verify(ROOT/'dist/v0.6.1', '0.6.1')
        self.assertEqual(len(result['packages']), 7)

    def test_missing_macos_cannot_pass_as_complete_release(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for platform in ['windows', 'linux']:
                for suffix in module.SUFFIXES[platform]:
                    name = 'itgla-v0.6.1-' + suffix
                    shutil.copy2(ROOT/'dist/v0.6.1'/name, root/name)
            shutil.copy2(ROOT/'dist/v0.6.1/sha256sums', root/'sha256sums')
            with self.assertRaises(FileNotFoundError):
                module.verify(root, '0.6.1')

    def test_modified_download_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for suffix in module.SUFFIXES['windows']:
                name = 'itgla-v0.6.1-' + suffix
                (root/name).write_bytes((ROOT/'dist/v0.6.1'/name).read_bytes())
            module.verify(root, '0.6.1', 'windows', write=True)
            exe = root/'itgla-v0.6.1-windows-x86_64-setup.exe'
            with exe.open('ab') as stream:stream.write(b'changed-after-checksum')
            with self.assertRaisesRegex(ValueError, 'checksum'):
                module.verify(root, '0.6.1', 'windows')

    def test_zip_path_escape_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory)/'unsafe.zip'
            with zipfile.ZipFile(path, 'w') as archive:
                archive.writestr('../private-data', b'x'*2048)
            with self.assertRaisesRegex(ValueError, 'Unsafe ZIP path'):
                module.inspect(path)

    def test_incomplete_dmg_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory)/'truncated.dmg';path.write_bytes(b'0'*2048)
            with self.assertRaisesRegex(ValueError, 'DMG'):
                module.inspect(path)


if __name__ == '__main__':
    unittest.main()
