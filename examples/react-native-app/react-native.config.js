const path = require('path');

module.exports = {
  dependencies: {
    'react-native-explorerkit': {
      root: path.resolve(__dirname, '../../packages/react-native-explorerkit'),
    },
  },
  project: {
    ios: {
      automaticPodsInstallation: true,
    },
  },
};
