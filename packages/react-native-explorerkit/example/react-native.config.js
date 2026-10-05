const fs = require('fs');
const path = require('path');

const installedPackageRoot = path.join(
  __dirname,
  'node_modules/react-native-explorerkit'
);
const installedPackageStats = fs.lstatSync(installedPackageRoot, {
  throwIfNoEntry: false,
});
const isInstalledConsumer =
  installedPackageStats?.isDirectory() &&
  !installedPackageStats.isSymbolicLink();
const packageRoot = isInstalledConsumer
  ? installedPackageRoot
  : path.dirname(__dirname);

module.exports = {
  dependencies: {
    'react-native-explorerkit': {
      root: packageRoot,
    },
  },
  project: {
    ios: {
      automaticPodsInstallation: true,
    },
  },
};
