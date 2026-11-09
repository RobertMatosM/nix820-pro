"""Package publication guards, using synthetic files rather than private app data."""
import contextlib
import hashlib
import io
import pathlib
import runpy
import tarfile
import tempfile
import tomllib
import unittest

ROOT = pathlib.Path(__file__).resolve().parent.parent
VALIDATOR = runpy.run_path(str(ROOT / 'tools/check-package.py'))
VERSION = tomllib.loads((ROOT / 'Cargo.toml').read_text())['package']['version']


def archive_in(root, extra=None, symlink=False, private_owner=False, version=VERSION):
    prefix = f'nix820-pro-v{version}-linux-x86_64'
    archive = pathlib.Path(root) / (prefix + '.tar.gz')
    with tarfile.open(archive, 'w:gz') as output:
        for name in sorted(VALIDATOR['EXPECTED']):
            member = tarfile.TarInfo(prefix + '/' + name)
            data = b'\x7fELFsynthetic' if name == 'nix820-pro' else b'test content'
            member.mode = 0o755 if name == 'nix820-pro' else 0o644
            if name == 'README.md' and symlink:
                member.type = tarfile.SYMTYPE
                member.linkname = '/etc/passwd'
                output.addfile(member)
                continue
            if private_owner:
                member.uid = 1234
                member.uname = 'synthetic-owner'
            member.size = len(data)
            output.addfile(member, io.BytesIO(data))
        if extra:
            member = tarfile.TarInfo(prefix + '/' + extra)
            member.size = 4
            output.addfile(member, io.BytesIO(b'test'))
    archive.with_name(archive.name + '.sha256').write_text(
        hashlib.sha256(archive.read_bytes()).hexdigest() + '  ' + archive.name + '\n')
    return archive


class PackageGuards(unittest.TestCase):
    def test_allowlist_rejects_private_files_unsafe_paths_links_and_host_metadata(self):
        with tempfile.TemporaryDirectory(prefix='nix820-package-guards-') as root:
            with contextlib.redirect_stdout(io.StringIO()):
                VALIDATOR['check'](archive_in(root))
            for kwargs in [{'extra': 'capture.pcapng'}, {'extra': 'profiles.json'},
                           {'extra': '../outside'}, {'symlink': True}, {'private_owner': True}]:
                with self.subTest(kwargs=kwargs), self.assertRaises(AssertionError):
                    VALIDATOR['check'](archive_in(root, **kwargs))

    def test_modified_archive_and_wrong_version_are_rejected(self):
        with tempfile.TemporaryDirectory(prefix='nix820-package-guards-') as root:
            archive = archive_in(root)
            with archive.open('ab') as output:
                output.write(b'tampered')
            with self.assertRaises(AssertionError):
                VALIDATOR['check'](archive)
            with self.assertRaises(AssertionError):
                VALIDATOR['check'](archive_in(root, version='9.9.9'))


if __name__ == '__main__':
    unittest.main()
