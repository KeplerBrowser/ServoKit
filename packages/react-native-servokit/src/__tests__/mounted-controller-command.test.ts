import { expect, test } from 'bun:test';

import { sendMountedControllerCommand } from '../MountedControllerCommand';

test('forwards opaque controller command JSON once without changing it', () => {
  const nativeView = {};
  const commandJson = '{ "version": 1, "command": "reload", "opaque": " keep spacing " }';
  const calls: Array<Readonly<{ nativeView: object; commandJson: string }>> = [];
  const nativeServoViewCommands = {
    sendControllerCommand(view: object, json: string) {
      calls.push({ nativeView: view, commandJson: json });
    },
  };

  expect(
    sendMountedControllerCommand(nativeView, commandJson, nativeServoViewCommands)
  ).toBe(true);
  expect(calls).toEqual([{ nativeView, commandJson }]);
});

test('does not dispatch without a mounted native view', () => {
  let dispatchCount = 0;
  const nativeServoViewCommands = {
    sendControllerCommand() {
      dispatchCount += 1;
    },
  };

  expect(
    sendMountedControllerCommand(null, '{}', nativeServoViewCommands)
  ).toBe(false);
  expect(dispatchCount).toBe(0);
});

test('returns false when native command dispatch throws', () => {
  let dispatchCount = 0;
  const nativeServoViewCommands = {
    sendControllerCommand() {
      dispatchCount += 1;
      throw new Error('dispatch failed');
    },
  };

  expect(
    sendMountedControllerCommand({}, '{}', nativeServoViewCommands)
  ).toBe(false);
  expect(dispatchCount).toBe(1);
});
