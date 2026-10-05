# Package test app

The `react-native-explorerkit` package's own test app. Unlike the public
[React Native example](../../../examples/react-native-app/README.md), it has
shortcuts to every shared test page and exercises navigation policy,
JavaScript evaluation, and dialogs.

It is used by:

- the [Maestro](https://maestro.mobile.dev) end-to-end flows in `../e2e`
  (`bun run --cwd packages/react-native-explorerkit test:e2e:ios`);
- the Android unit tests of the React Native adapter
  (`bun run --cwd packages/react-native-explorerkit test:native:android`).

On Android, the app bundles the test pages and serves them on port 8481 from
inside the app. On iOS, serve them from your machine:

```sh
bun run --cwd packages/react-native-explorerkit example:fixtures:ios
```

Run commands and expected results are in
[Testing and validation](../../../docs/reference/testing.md).
