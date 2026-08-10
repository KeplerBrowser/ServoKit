import { expect, test } from 'bun:test';

import {
  createJavaScriptEvaluationLifecycleError,
  JavaScriptEvaluationRegistry,
} from '../JavaScriptEvaluationRegistry';

test('resolves pending JavaScript evaluations with value JSON strings', async () => {
  const registry = new JavaScriptEvaluationRegistry();
  const { evaluationId, promise } = registry.createPendingEvaluation();

  expect(registry.pendingCount()).toBe(1);
  expect(
    registry.settleEvaluation({
      evaluationId,
      ok: true,
      valueJson: '{"type":"string","value":"Servo"}',
      errorType: null,
    })
  ).toBe(true);

  await expect(promise).resolves.toBe('{"type":"string","value":"Servo"}');
  expect(registry.pendingCount()).toBe(0);
});

test('rejects pending JavaScript evaluations with native error categories', async () => {
  const registry = new JavaScriptEvaluationRegistry();
  const { evaluationId, promise } = registry.createPendingEvaluation();

  expect(
    registry.settleEvaluation({
      evaluationId,
      ok: false,
      valueJson: null,
      errorType: 'EvaluationFailure',
    })
  ).toBe(true);

  const error = await promise.catch((reason: unknown) => reason as Error & { code?: string });
  expect(error.name).toBe('JavaScriptEvaluationError');
  expect(error.message).toBe(
    'JavaScript evaluation failed: EvaluationFailure'
  );
  expect(error.code).toBe('EvaluationFailure');
  expect(registry.pendingCount()).toBe(0);
});

test('rejects all pending JavaScript evaluations during unmount cleanup', async () => {
  const registry = new JavaScriptEvaluationRegistry();
  const first = registry.createPendingEvaluation();
  const second = registry.createPendingEvaluation();

  registry.rejectAll(
    createJavaScriptEvaluationLifecycleError(
      'ServoView unmounted before JavaScript evaluation completed.'
    )
  );

  const firstError = await first.promise.catch(
    (reason: unknown) => reason as Error & { code?: string }
  );
  const secondError = await second.promise.catch(
    (reason: unknown) => reason as Error & { code?: string }
  );

  expect(firstError.name).toBe('JavaScriptEvaluationLifecycleError');
  expect(firstError.code).toBe('LifecycleError');
  expect(secondError.message).toBe(
    'ServoView unmounted before JavaScript evaluation completed.'
  );
  expect(registry.pendingCount()).toBe(0);
});
