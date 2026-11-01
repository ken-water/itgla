#!/usr/bin/env python3
"""Inspect ELF ABI requirements and fail before creating incompatible packages."""
import argparse
import json
import re
import subprocess
import tempfile
from pathlib import Path


def requirements(binary, maximum='2.35'):
    output = subprocess.check_output(['readelf', '--version-info', str(binary)], text=True)
    versions = set(re.findall(r'\bGLIBC_(\d+(?:\.\d+)+)\b', output))
    if not versions:
        raise RuntimeError('Cannot identify glibc requirements; expected a dynamically linked Linux ELF')
    key = lambda v: tuple(map(int, v.split('.')))
    actual = max(versions, key=key)
    if key(actual) > key(maximum):
        raise RuntimeError(f'{binary} requires GLIBC {actual}; maximum supported baseline is {maximum}. '
                           'Rebuild with scripts/build-linux-baseline.sh; do not relabel a host-native binary.')
    return {'binary': str(binary), 'maximum_glibc': maximum, 'required_glibc': actual}


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('binary', type=Path)
    parser.add_argument('--maximum', default='2.35')
    parser.add_argument('--appimage', action='store_true')
    args = parser.parse_args()
    if args.appimage:
        with tempfile.TemporaryDirectory(prefix='itgla-appimage-abi-') as directory:
            subprocess.run([str(args.binary.resolve()), '--appimage-extract'], cwd=directory,
                           check=True, stdout=subprocess.DEVNULL)
            root = Path(directory) / 'squashfs-root'
            primary = requirements(root / 'usr/bin/itgla', args.maximum)
            libraries = []
            for path in sorted(root.rglob('*')):
                if not path.is_file() or path.is_symlink():
                    continue
                with path.open('rb') as stream:
                    if stream.read(4) != b'\x7fELF':
                        continue
                result = requirements(path, args.maximum)
                result['binary'] = str(path.relative_to(root))
                libraries.append(result)
            print(json.dumps({'status': 'pass', 'primary': primary['required_glibc'],
                              'elf_files': libraries}, indent=2))
    else:
        print(json.dumps(requirements(args.binary, args.maximum)))
