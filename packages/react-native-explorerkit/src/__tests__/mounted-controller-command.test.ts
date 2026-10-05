import { expect, test } from 'bun:test';

import { sendMountedControllerCommand } from '../MountedControllerCommand';

test('forwards opaque controller command JSON once without changing it', () => {
  const nativeView = {};
  const commandJson = '{ "version": 1, "command": "reload", "opaque": " keep spacing " }';
  const calls: Array<Readonly<{ nativeView: object; commandJson: string }>> = [];
  const nativeExplorerViewCommands = {
    sendControllerCommand(view: object, json: string) {
      calls.push({ nativeView: view, commandJson: json });
    },
  };

  expect(
    sendMountedControllerCommand(nativeView, commandJson, nativeExplorerViewCommands)
  ).toBe(true);
  expect(calls).toEqual([{ nativeView, commandJson }]);
});

test('does not dispatch without a mounted native view', () => {
  let dispatchCount = 0;
  const nativeExplorerViewCommands = {
    sendControllerCommand() {
      dispatchCount += 1;
    },
  };

  expect(
    sendMountedControllerCommand(null, '{}', nativeExplorerViewCommands)
  ).toBe(false);
  expect(dispatchCount).toBe(0);
});

test('returns false when native command dispatch throws', () => {
  let dispatchCount = 0;
  const nativeExplorerViewCommands = {
    sendControllerCommand() {
      dispatchCount += 1;
      throw new Error('dispatch failed');
    },
  };

  expect(
    sendMountedControllerCommand({}, '{}', nativeExplorerViewCommands)
  ).toBe(false);
  expect(dispatchCount).toBe(1);
});
