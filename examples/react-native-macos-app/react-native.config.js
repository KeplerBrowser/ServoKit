const path = require('path');

module.exports = {
  reactNativePath: path.resolve(__dirname, 'node_modules/react-native-macos'),
  dependencies: {
    'react-native-servokit': {
      root: path.resolve(__dirname, '../../packages/react-native-servokit'),
    },
  },
};
