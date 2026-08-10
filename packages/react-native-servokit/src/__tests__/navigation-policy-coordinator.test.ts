import { expect, test } from 'bun:test';

import { NavigationPolicyCoordinator } from '../NavigationPolicyCoordinator';

type PendingTimer = Readonly<{
  id: number;
  callback: () => void;
  timeoutMs: number;
}>;

function createTimerApi() {
  let nextId = 0;
  const pending = new Map<number, PendingTimer>();
  return {
    pending,
    api: {
      setTimeout(callback: () => void, timeoutMs: number) {
        const timer = Object.freeze({ id: ++nextId, callback, timeoutMs });
        pending.set(timer.id, timer);
        return timer as unknown as ReturnType<typeof setTimeout>;
      },
      clearTimeout(timeout: ReturnType<typeof setTimeout>) {
        pending.delete((timeout as unknown as PendingTimer).id);
      },
    },
    fireFirst() {
      const timer = pending.values().next().value;
      if (timer) {
        timer.callback();
      }
    },
  };
}

async function flushMicrotasks() {
  await Promise.resolve();
  await Promise.resolve();
}

test('allows navigation when the policy callback resolves true', async () => {
  const decisions: Array<Readonly<{ navigationId: string; allow: boolean }>> = [];
  const timers = createTimerApi();
  const coordinator = new NavigationPolicyCoordinator(
    (navigationId, allow) => decisions.push(Object.freeze({ navigationId, allow })),
    5000,
    true,
    timers.api,
    () => {}
  );

  coordinator.handleRequest(
    { navigationId: 'navigation-1', url: 'https://servo.org/' },
    ({ url }) => url === 'https://servo.org/'
  );

  await flushMicrotasks();
  expect(decisions).toEqual([{ navigationId: 'navigation-1', allow: true }]);
  expect(coordinator.pendingCount()).toBe(0);
  expect(timers.pending.size).toBe(0);
});

test('denies navigation when the policy callback resolves false', async () => {
  const decisions: Array<Readonly<{ navigationId: string; allow: boolean }>> = [];
  const timers = createTimerApi();
  const coordinator = new NavigationPolicyCoordinator(
    (navigationId, allow) => decisions.push(Object.freeze({ navigationId, allow })),
    5000,
    true,
    timers.api,
    () => {}
  );

  coordinator.handleRequest(
    { navigationId: 'navigation-2', url: 'https://example.com/blocked-by-policy' },
    () => false
  );

  await flushMicrotasks();
  expect(decisions).toEqual([{ navigationId: 'navigation-2', allow: false }]);
  expect(coordinator.pendingCount()).toBe(0);
});

test('falls back to allow when the policy callback is missing', () => {
  const decisions: Array<Readonly<{ navigationId: string; allow: boolean }>> = [];
  const timers = createTimerApi();
  const coordinator = new NavigationPolicyCoordinator(
    (navigationId, allow) => decisions.push(Object.freeze({ navigationId, allow })),
    5000,
    true,
    timers.api,
    () => {}
  );

  coordinator.handleRequest(
    { navigationId: 'navigation-3', url: 'https://servo.org/' },
    undefined
  );

  expect(decisions).toEqual([{ navigationId: 'navigation-3', allow: true }]);
  expect(coordinator.pendingCount()).toBe(0);
  expect(timers.pending.size).toBe(0);
});

test('falls back to allow when the policy callback throws', async () => {
  const decisions: Array<Readonly<{ navigationId: string; allow: boolean }>> = [];
  const timers = createTimerApi();
  const coordinator = new NavigationPolicyCoordinator(
    (navigationId, allow) => decisions.push(Object.freeze({ navigationId, allow })),
    5000,
    true,
    timers.api,
    () => {}
  );

  coordinator.handleRequest(
    { navigationId: 'navigation-4', url: 'https://servo.org/' },
    () => {
      throw new Error('policy failed');
    }
  );

  expect(decisions).toEqual([{ navigationId: 'navigation-4', allow: true }]);
  expect(coordinator.pendingCount()).toBe(0);
  expect(timers.pending.size).toBe(0);
});

test('falls back to allow when the policy callback rejects', async () => {
  const decisions: Array<Readonly<{ navigationId: string; allow: boolean }>> = [];
  const timers = createTimerApi();
  const coordinator = new NavigationPolicyCoordinator(
    (navigationId, allow) => decisions.push(Object.freeze({ navigationId, allow })),
    5000,
    true,
    timers.api,
    () => {}
  );

  coordinator.handleRequest(
    { navigationId: 'navigation-5', url: 'https://servo.org/' },
    () => Promise.reject(new Error('policy rejected'))
  );

  await flushMicrotasks();
  expect(decisions).toEqual([{ navigationId: 'navigation-5', allow: true }]);
  expect(coordinator.pendingCount()).toBe(0);
});

test('falls back to allow when the policy callback times out', () => {
  const decisions: Array<Readonly<{ navigationId: string; allow: boolean }>> = [];
  const timers = createTimerApi();
  const coordinator = new NavigationPolicyCoordinator(
    (navigationId, allow) => decisions.push(Object.freeze({ navigationId, allow })),
    5000,
    true,
    timers.api,
    () => {}
  );

  coordinator.handleRequest(
    { navigationId: 'navigation-6', url: 'https://servo.org/' },
    () => new Promise<boolean>(() => {})
  );

  expect(decisions).toEqual([]);
  expect(coordinator.pendingCount()).toBe(1);
  timers.fireFirst();
  expect(decisions).toEqual([{ navigationId: 'navigation-6', allow: true }]);
  expect(coordinator.pendingCount()).toBe(0);
});

test('cleans up an iOS timeout without sending a competing allow decision', async () => {
  const decisions: Array<Readonly<{ navigationId: string; allow: boolean }>> = [];
  const timers = createTimerApi();
  let resolvePolicy: (allow: boolean) => void = () => {};
  const coordinator = new NavigationPolicyCoordinator(
    (navigationId, allow) => decisions.push(Object.freeze({ navigationId, allow })),
    5000,
    false,
    timers.api,
    () => {}
  );

  coordinator.handleRequest(
    { navigationId: 'navigation-ios', url: 'https://servo.org/' },
    () => new Promise<boolean>((resolve) => {
      resolvePolicy = resolve;
    })
  );

  timers.fireFirst();
  expect(decisions).toEqual([]);
  expect(coordinator.pendingCount()).toBe(0);
  expect(timers.pending.size).toBe(0);

  resolvePolicy(false);
  await flushMicrotasks();
  expect(decisions).toEqual([]);
});
