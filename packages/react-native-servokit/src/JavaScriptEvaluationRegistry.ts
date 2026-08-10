export type NativeJavaScriptEvaluationResult = Readonly<{
  evaluationId: string;
  ok: boolean;
  valueJson: string | null;
  errorType: string | null;
}>;

type PendingJavaScriptEvaluation = Readonly<{
  resolve: (value: string) => void;
  reject: (error: Error) => void;
}>;

export class JavaScriptEvaluationRegistry {
  private nextEvaluationId = 0;
  private readonly pending = new Map<string, PendingJavaScriptEvaluation>();

  createPendingEvaluation(): Readonly<{
    evaluationId: string;
    promise: Promise<string>;
  }> {
    const evaluationId = `evaluation-${++this.nextEvaluationId}`;
    const promise = new Promise<string>((resolve, reject) => {
      this.pending.set(evaluationId, { resolve, reject });
    });

    return Object.freeze({ evaluationId, promise });
  }

  rejectPendingEvaluation(evaluationId: string, error: Error): boolean {
    const pending = this.pending.get(evaluationId);
    if (!pending) {
      return false;
    }

    this.pending.delete(evaluationId);
    pending.reject(error);
    return true;
  }

  settleEvaluation(result: NativeJavaScriptEvaluationResult): boolean {
    const pending = this.pending.get(result.evaluationId);
    if (!pending) {
      return false;
    }

    this.pending.delete(result.evaluationId);
    if (result.ok && typeof result.valueJson === 'string') {
      pending.resolve(result.valueJson);
      return true;
    }

    pending.reject(createJavaScriptEvaluationError(result.errorType));
    return true;
  }

  rejectAll(error: Error): void {
    const pendingEntries = [...this.pending.values()];
    this.pending.clear();
    pendingEntries.forEach((pending) => pending.reject(error));
  }

  pendingCount(): number {
    return this.pending.size;
  }
}

export function createJavaScriptEvaluationError(errorType: string | null): Error {
  const category =
    typeof errorType === 'string' && errorType.length > 0 ? errorType : 'InternalError';
  const error = new Error(`JavaScript evaluation failed: ${category}`) as Error & {
    code?: string;
    name: string;
  };
  error.name = 'JavaScriptEvaluationError';
  error.code = category;
  return error;
}

export function createJavaScriptEvaluationLifecycleError(message: string): Error {
  const error = new Error(message) as Error & {
    code?: string;
    name: string;
  };
  error.name = 'JavaScriptEvaluationLifecycleError';
  error.code = 'LifecycleError';
  return error;
}
