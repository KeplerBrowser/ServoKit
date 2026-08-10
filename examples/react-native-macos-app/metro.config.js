const path = require('path');
const { getDefaultConfig } = require('@react-native/metro-config');
const { withMetroConfig } = require('react-native-monorepo-config');

const root = path.resolve(__dirname, '../..');
const config = withMetroConfig(getDefaultConfig(__dirname), {
  root,
  dirname: __dirname,
});
const reactNativeMacOS = path.resolve(
  __dirname,
  'node_modules/react-native-macos',
);
const resolveRequest = config.resolver.resolveRequest;

config.resolver.extraNodeModules = {
  ...config.resolver.extraNodeModules,
  '@babel/runtime': path.resolve(__dirname, 'node_modules/@babel/runtime'),
  'react-native-macos': reactNativeMacOS,
};
config.resolver.resolveRequest = (context, moduleName, platform) => {
  if (moduleName === 'react' || moduleName.startsWith('react/')) {
    return resolveRequest(
      context,
      require.resolve(moduleName, { paths: [__dirname] }),
      platform,
    );
  }

  const target =
    platform === 'macos' &&
    (moduleName === 'react-native' || moduleName.startsWith('react-native/'))
      ? moduleName.replace('react-native', 'react-native-macos')
      : moduleName;

  return resolveRequest(context, target, platform);
};

module.exports = config;
