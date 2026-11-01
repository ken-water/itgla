#!/usr/bin/env python3
"""Verify an exact release artifact set, hashes, formats and portable PE metadata."""
import argparse
import hashlib
import json
import struct
import zipfile
from pathlib import Path

SUFFIXES = {
    'windows': ['windows-x86_64-setup.exe', 'windows-x86_64.zip'],
    'linux': ['linux-x86_64.deb', 'linux-x86_64.rpm', 'linux-x86_64.AppImage'],
    'macos': ['macos-arm64.dmg', 'macos-x86_64.dmg'],
}


def check_pe(data):
    if data[:2] != b'MZ':
        raise ValueError('Missing Windows executable header')
    offset = struct.unpack_from('<I', data, 0x3c)[0]
    if data[offset:offset+4] != b'PE\0\0':
        raise ValueError('Invalid PE signature')
    machine = struct.unpack_from('<H', data, offset+4)[0]
    subsystem = struct.unpack_from('<H', data, offset+24+68)[0]
    if machine != 0x8664 or subsystem != 2:
        raise ValueError('Portable program must be an x86_64 Windows GUI executable')


def inspect(path):
    data = path.read_bytes()
    if len(data) < 1024:
        raise ValueError(f'Empty/truncated package: {path.name}')
    if path.suffix == '.zip':
        with zipfile.ZipFile(path) as archive:
            if archive.testzip():
                raise ValueError('ZIP integrity failed')
            entries = archive.namelist()
            if len(entries) != len(set(entries)):
                raise ValueError('Duplicate ZIP entries')
            for name in entries:
                if name.startswith(('/', '\\')) or '..' in Path(name.replace('\\', '/')).parts:
                    raise ValueError(f'Unsafe ZIP path: {name}')
            exe = [n for n in entries if n.endswith('/itgla.exe')]
            if len(exe) != 1:
                raise ValueError('Expected exactly one portable program')
            check_pe(archive.read(exe[0]))
            prefix = exe[0].rsplit('/', 1)[0]
            for filename in ['itgla.ico', 'README.md', 'CHANGELOG.md', 'WINDOWS-README.md']:
                if prefix + '/' + filename not in entries:
                    raise ValueError(f'Missing portable package file: {filename}')
            if any(n.lower().endswith(('.db', '.db-wal', '.db-shm', '.pdb')) for n in entries):
                raise ValueError('User data/debug artifacts found in release ZIP')
    elif path.suffix == '.exe':
        if data[:2] != b'MZ' or b'\xef\xbe\xad\xdeNullsoftInst' not in data[:1024*1024]:
            raise ValueError('Expected NSIS installer')
    elif path.suffix == '.deb' and data[:8] != b'!<arch>\n':
        raise ValueError('Invalid Debian archive')
    elif path.suffix == '.rpm' and data[:4] != b'\xed\xab\xee\xdb':
        raise ValueError('Invalid RPM archive')
    elif path.suffix == '.AppImage' and (data[:4] != b'\x7fELF' or data[8:11] != b'AI\x02'):
        raise ValueError('Invalid type-2 AppImage')
    elif path.suffix == '.dmg' and data[-512:-508] != b'koly':
        raise ValueError('Invalid UDIF DMG trailer')
    return {'name': path.name, 'bytes': len(data), 'sha256': hashlib.sha256(data).hexdigest()}


def verify(directory, version, platform='all', write=False):
    suffixes = sum(SUFFIXES.values(), []) if platform == 'all' else SUFFIXES[platform]
    names = [f'itgla-v{version}-{suffix}' for suffix in suffixes]
    results = [inspect(directory / name) for name in names]
    manifest = directory / ('sha256sums' if platform == 'all' else f'sha256sums-{platform}')
    if write:
        manifest.write_text(''.join(f'{r["sha256"]}  {r["name"]}\n' for r in results))
    else:
        # A full release manifest may also verify a platform subset.
        if not manifest.exists() and platform != 'all':
            manifest = directory / 'sha256sums'
        hashes = {}
        for line in manifest.read_text().splitlines():
            digest, name = line.split(None, 1)
            if name in hashes:
                raise ValueError(f'Duplicate checksum entry: {name}')
            hashes[name] = digest
        for r in results:
            if hashes.get(r['name']) != r['sha256']:
                raise ValueError(f'Missing/mismatched checksum: {r["name"]}')
    return {'version': version, 'platform': platform, 'status': 'pass', 'packages': results}


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('directory', type=Path)
    parser.add_argument('--version', required=True)
    parser.add_argument('--platform', choices=['all', *SUFFIXES], default='all')
    parser.add_argument('--write-manifest', action='store_true')
    args = parser.parse_args()
    print(json.dumps(verify(args.directory, args.version, args.platform, args.write_manifest), indent=2))
