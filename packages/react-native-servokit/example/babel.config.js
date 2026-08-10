const path = require('path');
const { getConfig } = require('react-native-builder-bob/babel-config');

const root =
  require('./react-native.config').dependencies['react-native-servokit'].root;
const pkg = require(path.join(root, 'package.json'));

module.exports = getConfig(
  {
    presets: ['module:@react-native/babel-preset'],
  },
  { root, pkg }
);
