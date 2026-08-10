import { createHash } from 'node:crypto';
import {
  copyFileSync,
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  statSync,
  writeFileSync,
} from 'node:fs';
import { basename, dirname, join, resolve } from 'node:path';

const [
  metadataPath,
  binaryPath,
  outputPath,
  version,
  created,
  licensesPath,
  noticesPath,
] = process.argv.slice(2);
if (!noticesPath) {
  throw new Error(
    'usage: create-sbom.mjs <metadata> <binary> <output> <version> <created> <licenses> <notices>'
  );
}

const metadata = JSON.parse(readFileSync(metadataPath, 'utf8'));
const packageById = new Map(
  metadata.packages.map((cargoPackage) => [cargoPackage.id, cargoPackage])
);
const nodeById = new Map(metadata.resolve.nodes.map((node) => [node.id, node]));
const roots = metadata.packages.filter(
  (cargoPackage) => cargoPackage.name === 'servokit-host-desktop'
);
if (roots.length !== 1) {
  throw new Error(`expected one servokit-host-desktop package, got ${roots.length}`);
}

const reachable = new Set([roots[0].id]);
for (const packageId of reachable) {
  const node = nodeById.get(packageId);
  if (!node) {
    throw new Error(`Cargo metadata is missing resolve node ${packageId}`);
  }
  for (const dependency of node.deps) {
    if (dependency.dep_kinds.some((kind) => kind.kind === null)) {
      reachable.add(dependency.pkg);
    }
  }
}

const packages = [...reachable]
  .map((packageId) => {
    const cargoPackage = packageById.get(packageId);
    if (!cargoPackage) {
      throw new Error(`Cargo metadata is missing package ${packageId}`);
    }
    return cargoPackage;
  })
  .sort((left, right) => left.id.localeCompare(right.id));
const id = (cargoPackage) =>
  `SPDXRef-${cargoPackage.id}`.replace(/[^A-Za-z0-9.-]/g, '-');
const binarySha256 = createHash('sha256')
  .update(readFileSync(binaryPath))
  .digest('hex');
const runtimeId = 'SPDXRef-ServoKitRuntime';

mkdirSync(licensesPath, { recursive: true });
const noticeRows = [];
for (const cargoPackage of packages) {
  const packageDirectory = dirname(cargoPackage.manifest_path);
  const candidates = new Set(
    readdirSync(packageDirectory)
      .filter((entry) => /^(?:licen[cs]e|copying|notice)(?:[._-].*)?$/i.test(entry))
      .map((entry) => join(packageDirectory, entry))
  );
  if (cargoPackage.license_file) {
    candidates.add(resolve(packageDirectory, cargoPackage.license_file));
  }

  const packageSuffix = createHash('sha256')
    .update(cargoPackage.id)
    .digest('hex')
    .slice(0, 8);
  const destination = join(
    licensesPath,
    `${cargoPackage.name}-${cargoPackage.version}-${packageSuffix}`
  );
  let copied = 0;
  for (const candidate of [...candidates].sort()) {
    if (!existsSync(candidate) || !statSync(candidate).isFile()) {
      continue;
    }
    mkdirSync(destination, { recursive: true });
    copyFileSync(candidate, join(destination, basename(candidate)));
    copied += 1;
  }
  noticeRows.push(
    `| ${cargoPackage.name} | ${cargoPackage.version} | ${cargoPackage.license ?? 'NOASSERTION'} | ${cargoPackage.source ?? 'local source'} | ${copied} |`
  );
}

writeFileSync(
  noticesPath,
  [
    '# Third-Party Notices',
    '',
    'This file records the version-locked Cargo runtime graph linked into the ServoKit macOS framework. Corresponding license and notice files found in each package are under `LICENSES/Cargo`.',
    '',
    '| Package | Version | Declared license | Source | License files |',
    '| --- | --- | --- | --- | ---: |',
    ...noticeRows,
    '',
  ].join('\n')
);

const document = {
  spdxVersion: 'SPDX-2.3',
  dataLicense: 'CC0-1.0',
  SPDXID: 'SPDXRef-DOCUMENT',
  name: `ServoKit-macOS-${version}`,
  documentNamespace: `https://github.com/KeplerBrowser/ServoKit/sbom/macos/${version}/${binarySha256}`,
  creationInfo: {
    created,
    creators: ['Tool: ServoKit distribution/macos/create-sbom.mjs'],
  },
  packages: [
    {
      SPDXID: runtimeId,
      name: 'ServoKit',
      versionInfo: version,
      downloadLocation: 'NOASSERTION',
      filesAnalyzed: false,
      licenseConcluded: 'NOASSERTION',
      licenseDeclared: 'MIT',
      checksums: [{ algorithm: 'SHA256', checksumValue: binarySha256 }],
      packageFileName: 'ServoKit.xcframework',
      primaryPackagePurpose: 'LIBRARY',
    },
    ...packages.map((pkg) => ({
      SPDXID: id(pkg),
      name: pkg.name,
      versionInfo: pkg.version,
      downloadLocation: pkg.source ?? 'NOASSERTION',
      filesAnalyzed: false,
      licenseConcluded: 'NOASSERTION',
      licenseDeclared: pkg.license ?? 'NOASSERTION',
      externalRefs: [
        {
          referenceCategory: 'PACKAGE-MANAGER',
          referenceType: 'purl',
          referenceLocator: `pkg:cargo/${encodeURIComponent(pkg.name)}@${pkg.version}`,
        },
      ],
    })),
  ],
  relationships: [
    {
      spdxElementId: 'SPDXRef-DOCUMENT',
      relationshipType: 'DESCRIBES',
      relatedSpdxElement: runtimeId,
    },
    ...packages.map((pkg) => ({
      spdxElementId: runtimeId,
      relationshipType: 'DEPENDS_ON',
      relatedSpdxElement: id(pkg),
    })),
  ],
  annotations: [
    {
      annotationDate: created,
      annotationType: 'OTHER',
      annotator: 'Tool: ServoKit distribution/macos/create-sbom.mjs',
      comment: `Universal framework binary size: ${statSync(binaryPath).size} bytes`,
    },
  ],
};

writeFileSync(outputPath, `${JSON.stringify(document, null, 2)}\n`);
