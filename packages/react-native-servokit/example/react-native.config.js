const fs = require('fs');
const path = require('path');

const installedPackageRoot = path.join(
  __dirname,
  'node_modules/react-native-servokit'
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
    'react-native-servokit': {
      root: packageRoot,
    },
  },
  project: {
    ios: {
      automaticPodsInstallation: true,
    },
  },
};
