import { expect, test } from 'bun:test';

import { JavaScriptDialogCoordinator } from '../JavaScriptDialogCoordinator';

function createCoordinator() {
  const resolutions: Array<
    Readonly<{ dialogId: string; confirmed: boolean; promptValue: string | null }>
  > = [];
  const errors: unknown[] = [];
  const coordinator = new JavaScriptDialogCoordinator(
    (dialogId, confirmed, promptValue) => {
      resolutions.push(Object.freeze({ dialogId, confirmed, promptValue }));
    },
    (_message, error) => errors.push(error)
  );
  return { coordinator, resolutions, errors };
}

test('confirms an alert dialog exactly once', () => {
  const { coordinator, resolutions } = createCoordinator();
  let request: Parameters<NonNullable<Parameters<typeof coordinator.handleRequest>[1]>>[0] | null =
    null;

  coordinator.handleRequest(
    {
      dialogId: 'dialog-1',
      kind: 'alert',
      message: 'Hello',
      defaultValue: null,
    },
    (nextRequest) => {
      request = nextRequest;
    }
  );

  expect(request?.kind).toBe('alert');
  request?.confirm();
  request?.dismiss();

  expect(resolutions).toEqual([
    { dialogId: 'dialog-1', confirmed: true, promptValue: null },
  ]);
  expect(coordinator.pendingCount()).toBe(0);
});

test('dismisses a confirm dialog', () => {
  const { coordinator, resolutions } = createCoordinator();

  coordinator.handleRequest(
    {
      dialogId: 'dialog-2',
      kind: 'confirm',
      message: 'Continue?',
      defaultValue: null,
    },
    (request) => request.dismiss()
  );

  expect(resolutions).toEqual([
    { dialogId: 'dialog-2', confirmed: false, promptValue: null },
  ]);
  expect(coordinator.pendingCount()).toBe(0);
});

test('confirms a prompt with the supplied value', () => {
  const { coordinator, resolutions } = createCoordinator();

  coordinator.handleRequest(
    {
      dialogId: 'dialog-3',
      kind: 'prompt',
      message: 'Name?',
      defaultValue: 'Servo',
    },
    (request) => request.confirm('ServoKit')
  );

  expect(resolutions).toEqual([
    { dialogId: 'dialog-3', confirmed: true, promptValue: 'ServoKit' },
  ]);
});

test('confirms a prompt with its default value when none is supplied', () => {
  const { coordinator, resolutions } = createCoordinator();

  coordinator.handleRequest(
    {
      dialogId: 'dialog-4',
      kind: 'prompt',
      message: 'Name?',
      defaultValue: 'Servo',
    },
    (request) => request.confirm()
  );

  expect(resolutions).toEqual([
    { dialogId: 'dialog-4', confirmed: true, promptValue: 'Servo' },
  ]);
});

test('dismisses when the React Native handler is missing', () => {
  const { coordinator, resolutions } = createCoordinator();

  coordinator.handleRequest(
    {
      dialogId: 'dialog-5',
      kind: 'confirm',
      message: 'Continue?',
      defaultValue: null,
    },
    undefined
  );

  expect(resolutions).toEqual([
    { dialogId: 'dialog-5', confirmed: false, promptValue: null },
  ]);
  expect(coordinator.pendingCount()).toBe(0);
});

test('dismisses and rethrows when the handler throws', () => {
  const { coordinator, resolutions, errors } = createCoordinator();
  const thrown = new Error('handler failed');

  expect(() =>
    coordinator.handleRequest(
      {
        dialogId: 'dialog-6',
        kind: 'alert',
        message: 'Hello',
        defaultValue: null,
      },
      () => {
        throw thrown;
      }
    )
  ).toThrow(thrown);

  expect(errors).toEqual([thrown]);
  expect(resolutions).toEqual([
    { dialogId: 'dialog-6', confirmed: false, promptValue: null },
  ]);
  expect(coordinator.pendingCount()).toBe(0);
});

test('drops native dismissals without sending a duplicate resolution', () => {
  const { coordinator, resolutions } = createCoordinator();
  let request: Parameters<NonNullable<Parameters<typeof coordinator.handleRequest>[1]>>[0] | null =
    null;

  coordinator.handleRequest(
    {
      dialogId: 'dialog-7',
      kind: 'confirm',
      message: 'Continue?',
      defaultValue: null,
    },
    (nextRequest) => {
      request = nextRequest;
    }
  );

  coordinator.handleDismissed('dialog-7');
  request?.confirm();

  expect(resolutions).toEqual([]);
  expect(coordinator.pendingCount()).toBe(0);
});
