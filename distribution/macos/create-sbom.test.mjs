import { expect, test } from 'bun:test';
import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';

test('SBOM includes only the reachable Cargo runtime graph', () => {
  const directory = mkdtempSync(join(tmpdir(), 'servokit-sbom-'));
  try {
    const rootDirectory = join(directory, 'root');
    const dependencyDirectory = join(directory, 'dependency');
    const developmentDirectory = join(directory, 'development');
    const buildDirectory = join(directory, 'build');
    for (const packageDirectory of [
      rootDirectory,
      dependencyDirectory,
      developmentDirectory,
      buildDirectory,
    ]) {
      mkdirSync(packageDirectory);
      writeFileSync(join(packageDirectory, 'Cargo.toml'), '[package]\n');
      writeFileSync(join(packageDirectory, 'LICENSE'), 'license text\n');
    }

    const rootId = 'path+file:///root#servokit-host-desktop@0.1.0';
    const dependencyId = 'registry+https://example.invalid#runtime@1.0.0';
    const developmentId = 'registry+https://example.invalid#dev-only@1.0.0';
    const buildId = 'registry+https://example.invalid#build-only@1.0.0';
    const metadata = {
      packages: [
        {
          id: rootId,
          name: 'servokit-host-desktop',
          version: '0.1.0',
          source: null,
          license: 'MIT',
          license_file: null,
          manifest_path: join(rootDirectory, 'Cargo.toml'),
        },
        {
          id: dependencyId,
          name: 'runtime',
          version: '1.0.0',
          source: 'registry+https://example.invalid',
          license: 'Apache-2.0',
          license_file: null,
          manifest_path: join(dependencyDirectory, 'Cargo.toml'),
        },
        {
          id: developmentId,
          name: 'dev-only',
          version: '1.0.0',
          source: 'registry+https://example.invalid',
          license: 'MIT',
          license_file: null,
          manifest_path: join(developmentDirectory, 'Cargo.toml'),
        },
        {
          id: buildId,
          name: 'build-only',
          version: '1.0.0',
          source: 'registry+https://example.invalid',
          license: 'MIT',
          license_file: null,
          manifest_path: join(buildDirectory, 'Cargo.toml'),
        },
      ],
      resolve: {
        nodes: [
          {
            id: rootId,
            deps: [
              {
                pkg: dependencyId,
                dep_kinds: [{ kind: null, target: null }],
              },
              {
                pkg: developmentId,
                dep_kinds: [{ kind: 'dev', target: null }],
              },
              {
                pkg: buildId,
                dep_kinds: [{ kind: 'build', target: null }],
              },
            ],
          },
          { id: dependencyId, deps: [] },
          { id: developmentId, deps: [] },
          { id: buildId, deps: [] },
        ],
      },
    };
    const metadataPath = join(directory, 'metadata.json');
    const binaryPath = join(directory, 'ServoKit');
    const sbomPath = join(directory, 'ServoKit.spdx.json');
    const licensesPath = join(directory, 'licenses');
    const noticesPath = join(directory, 'THIRD-PARTY-NOTICES.md');
    writeFileSync(metadataPath, JSON.stringify(metadata));
    writeFileSync(binaryPath, 'binary');

    const result = spawnSync(
      'bun',
      [
        new URL('./create-sbom.mjs', import.meta.url).pathname,
        metadataPath,
        binaryPath,
        sbomPath,
        '0.1.0',
        '2026-01-01T00:00:00Z',
        licensesPath,
        noticesPath,
      ],
      { encoding: 'utf8' }
    );
    expect(result.status, result.stderr).toBe(0);

    const sbom = JSON.parse(readFileSync(sbomPath, 'utf8'));
    expect(sbom.packages.map((pkg) => pkg.name)).toEqual([
      'ServoKit',
      'servokit-host-desktop',
      'runtime',
    ]);
    expect(readFileSync(noticesPath, 'utf8')).not.toContain('dev-only');
    expect(readFileSync(noticesPath, 'utf8')).not.toContain('build-only');
    expect(
      readdirSync(licensesPath).some((entry) => entry.startsWith('dev-only-'))
    ).toBe(false);
    expect(
      readFileSync(noticesPath, 'utf8').includes(directory)
    ).toBe(false);
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});
