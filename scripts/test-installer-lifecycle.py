"""Exercise actual NSIS lifecycle with isolated registry, shortcut names and temp files.

Only fixtures compiled with ARP=Software\\ITGLAInstallerAcceptance20261101 and
SHORTCUT_NAME=ITGLA Installer Acceptance 20261101 may be supplied. No real workspace
or real installation is changed. Never point this script at a public setup EXE.
"""
import argparse
import ctypes
from ctypes import wintypes
import hashlib
import json
import os
from pathlib import Path
import stat
import subprocess
import tempfile
import time
import winreg

KEY = r'Software\ITGLAInstallerAcceptance20261101'


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def run(fixtures, output):
    manifest = json.loads((fixtures / 'fixture-manifest.json').read_text())
    assert manifest['arp'] == KEY
    assert manifest['default_directory'] == 'ITGLAInstallerAcceptance20261101'
    for filename, expected in manifest['fixtures'].items():
        assert digest(fixtures / filename) == expected
    # Refuse to overwrite another test's registry state.
    try:
        with winreg.OpenKey(winreg.HKEY_CURRENT_USER, KEY):
            raise RuntimeError('Test registry key already exists; inspect previous evidence before cleanup')
    except FileNotFoundError:
        pass
    work = Path(tempfile.mkdtemp(prefix='itgla-installer-lifecycle-'))
    install = work / 'Path with spaces' / 'ITGLA'
    workspace = work / 'workspace';workspace.mkdir()
    sentinel = workspace / 'itgla.db';sentinel.write_bytes(b'isolated-data-retention-sentinel')
    sentinel_hash = digest(sentinel)
    env = dict(os.environ, ITGLA_DATA_DIR=str(workspace))
    print('Isolated lifecycle directory:', work, flush=True)
    results = []
    def setup(version, expected=0, directory=False):
        executable = fixtures / f'fixture-{version}.exe'
        # Fail closed if a normal product installer was accidentally supplied.
        if executable.name not in ['fixture-1.0.0.exe', 'fixture-1.0.1.exe']:
            raise RuntimeError('Only isolated lifecycle fixture installers are allowed')
        args = [str(executable), '/S']
        if directory:
            args.append('/D=' + str(install))
        command = subprocess.list2cmdline(args[:2])
        if directory:
            # NSIS requires /D= followed by the unquoted remainder of the command line.
            command += ' /D=' + str(install)
        result = subprocess.run(command, env=env, timeout=60)
        assert result.returncode == expected, (version, result.returncode, expected)
    def record(name):
        assert digest(sentinel) == sentinel_hash
        results.append({'test': name, 'status': 'pass'})
    setup('1.0.0', directory=True)
    assert (install / 'itgla.exe').exists()
    with winreg.OpenKey(winreg.HKEY_CURRENT_USER, KEY) as key:
        assert winreg.QueryValueEx(key, 'InstallLocation')[0] == str(install)
        assert winreg.QueryValueEx(key, 'DisplayVersion')[0] == '1.0.0'
        assert winreg.QueryValueEx(key, 'QuietUninstallString')[0].startswith('"')
        assert winreg.QueryValueEx(key, 'EstimatedSize')[0] > 0
    record('fresh silent installation, spaces in path and complete registry metadata')
    (install / 'unknown-user-file.db').write_bytes(b'keep-me')
    record('desktop shortcut opt-in default')
    assert not (Path(os.environ['USERPROFILE']) / 'Desktop' / 'ITGLA Installer Acceptance 20261101.lnk').exists()
    setup('1.0.1')
    assert (install / 'README.md').read_text() == 'Installer fixture version 1.0.1'
    assert not (install / '.itgla-setup-backup').exists()
    record('upgrade reuses registered directory; successful backup cleanup')
    setup('1.0.1')
    record('same-version reinstall')
    setup('1.0.0', expected=4)
    assert (install / 'README.md').read_text() == 'Installer fixture version 1.0.1'
    record('silent downgrade rejected without changing installed files')
    running = subprocess.Popen([str(install / 'itgla.exe')], env=env)
    try:
        time.sleep(0.5)
        assert running.poll() is None
        setup('1.0.1', expected=2)
        result = subprocess.run(subprocess.list2cmdline([str(install / 'Uninstall.exe'), '/S']) + ' _?=' + str(install), env=env, timeout=60)
        assert result.returncode == 2, result.returncode
        record('running executable blocks install and uninstall')
    finally:
        running.terminate();running.wait(timeout=10)
    # Hold an existing document open with read sharing only: backup reads work,
    # replacement/restoration writes fail. This exercises real Windows errors.
    (install / 'README.md').write_bytes(b'user-edited-program-readme-for-failure-test')
    original = digest(install / 'README.md')
    kernel = ctypes.WinDLL('kernel32', use_last_error=True)
    kernel.CreateFileW.argtypes = [wintypes.LPCWSTR, wintypes.DWORD, wintypes.DWORD,
                                   ctypes.c_void_p, wintypes.DWORD, wintypes.DWORD, wintypes.HANDLE]
    kernel.CreateFileW.restype = wintypes.HANDLE
    kernel.CloseHandle.argtypes = [wintypes.HANDLE]
    lock = kernel.CreateFileW(str(install / 'README.md'), 0x80000000, 1, None, 3, 0, None)
    assert lock != ctypes.c_void_p(-1).value
    try:
        setup('1.0.1', expected=5)
        assert digest(install / 'README.md') == original
        recovery = install / '.itgla-setup-backup'
        assert recovery.exists(), 'Failed rollback must retain durable recovery copies'
        assert digest(recovery / 'README.md') == original
        record('failed replacement retains recoverable originals and data')
    finally:
        kernel.CloseHandle(lock)
    setup('1.0.1', expected=6)
    record('unresolved recovery directory blocks further replacement')
    # Recover fixture program files explicitly, without touching any workspace.
    for path in recovery.iterdir():
        if path.is_file():
            destination = install / path.name
            destination.write_bytes(path.read_bytes())
            path.chmod(stat.S_IWRITE | stat.S_IREAD)
            path.unlink()
    recovery.rmdir()
    result = subprocess.run(subprocess.list2cmdline([str(install / 'Uninstall.exe'), '/S']) + ' _?=' + str(install), env=env, timeout=60)
    assert result.returncode == 0, result.returncode
    assert not (install / 'itgla.exe').exists()
    assert (install / 'unknown-user-file.db').read_bytes() == b'keep-me'
    try:
        winreg.OpenKey(winreg.HKEY_CURRENT_USER, KEY)
        raise AssertionError('Uninstall registry key was retained')
    except FileNotFoundError:
        pass
    record('uninstall removes known programs/registry and preserves workspace and unknown files')
    output.write_text(json.dumps({'status': 'pass', 'work_directory': str(work), 'tests': results}, indent=2))
    print(json.dumps({'status': 'pass', 'checks': len(results), 'evidence': str(output)}))


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('fixtures', type=Path)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    run(args.fixtures.resolve(), args.output.resolve())
