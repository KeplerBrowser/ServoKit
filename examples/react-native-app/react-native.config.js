const path = require('path');

module.exports = {
  dependencies: {
    'react-native-servokit': {
      root: path.resolve(__dirname, '../../packages/react-native-servokit'),
    },
  },
  project: {
    ios: {
      automaticPodsInstallation: true,
    },
  },
};
