const path = require('path');
const { getDefaultConfig } = require('@react-native/metro-config');
const { withMetroConfig } = require('react-native-monorepo-config');

const packageRoot =
  require('./react-native.config').dependencies['react-native-servokit'].root;
const isWorkspace = packageRoot === path.dirname(__dirname);
const workspaceRoot = path.dirname(path.dirname(path.dirname(__dirname)));

/**
 * Metro configuration
 * https://facebook.github.io/metro/docs/configuration
 *
 * @type {import('metro-config').MetroConfig}
 */
const config = isWorkspace
  ? withMetroConfig(getDefaultConfig(__dirname), {
      root: workspaceRoot,
      dirname: __dirname,
    })
  : getDefaultConfig(__dirname);

config.resolver.extraNodeModules = {
  ...config.resolver.extraNodeModules,
  '@babel/runtime': path.resolve(__dirname, 'node_modules/@babel/runtime'),
};

module.exports = config;
