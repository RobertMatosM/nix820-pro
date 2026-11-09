"""Check archive contents and hashes before installing or publishing a package."""
import argparse
import hashlib
import pathlib
import tarfile
import tomllib

ROOT = pathlib.Path(__file__).resolve().parent.parent
EXPECTED = {'nix820-pro', 'README.md', 'INSTALL.md', 'LICENSE', 'THIRD_PARTY_NOTICES.md',
            'CHANGELOG.md', 'nix820-pro.desktop', 'nix820-pro.svg', '70-nix820-pro.rules',
            'install-user.sh', 'uninstall-user.sh', 'install-paths.sh', 'BUILDINFO.txt',
            'docs/images/keymap.png', 'docs/images/lighting.png'}


def check(archive):
    version = tomllib.loads((ROOT / 'Cargo.toml').read_text())['package']['version']
    prefix = archive.name.removesuffix('.tar.gz')
    assert prefix.startswith(f'nix820-pro-v{version}-linux-'), 'Package version/name mismatch'
    checksum = archive.with_name(archive.name + '.sha256').read_text().strip().split()
    assert checksum == [hashlib.sha256(archive.read_bytes()).hexdigest(), archive.name], 'Checksum mismatch'
    files = set()
    with tarfile.open(archive) as package:
        for member in package.getmembers():
            path = pathlib.PurePosixPath(member.name)
            assert path.parts[0] == prefix and '..' not in path.parts, 'Unsafe archive path'
            assert member.isdir() or member.isfile(), 'Links/devices are not package contents'
            assert member.uid == member.gid == 0 and not member.uname and not member.gname, 'Host owner metadata in archive'
            if member.isfile():
                relative = str(path.relative_to(prefix))
                assert relative in EXPECTED, f'Unexpected/private package member: {relative}'
                assert relative not in files, 'Duplicate archive member'
                assert member.size > 0, 'Empty package member'
                files.add(relative)
                if relative == 'nix820-pro':
                    assert member.mode & 0o111, 'Binary is not executable'
                    assert package.extractfile(member).read(4) == b'\x7fELF', 'Expected native Linux ELF'
        assert files == EXPECTED, f'Missing package files: {EXPECTED - files}'
    print(f'Verified {archive.name}: {len(files)} allowlisted files and SHA256.')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('archive', type=pathlib.Path)
    check(parser.parse_args().archive)
