export type NavigationPolicyRequest = Readonly<{
  url: string;
}>;

export type NavigationPolicyHandler = (
  request: NavigationPolicyRequest
) => boolean | Promise<boolean>;

type TimeoutHandle = ReturnType<typeof setTimeout>;

export type NavigationPolicyTimerApi = Readonly<{
  setTimeout(callback: () => void, timeoutMs: number): TimeoutHandle;
  clearTimeout(timeout: TimeoutHandle): void;
}>;

export type NativeNavigationPolicyRequest = Readonly<{
  navigationId: string;
  url: string;
}>;

export class NavigationPolicyCoordinator {
  private readonly pendingNavigationRequests = new Set<string>();
  private readonly pendingNavigationTimeouts = new Map<string, TimeoutHandle>();

  constructor(
    private readonly resolveNativeDecision: (
      navigationId: string,
      allow: boolean
    ) => void,
    private readonly timeoutMs: number,
    private readonly resolveNativeFallbackOnTimeout = true,
    private readonly timerApi: NavigationPolicyTimerApi = {
      setTimeout: (callback, timeoutMs) => setTimeout(callback, timeoutMs),
      clearTimeout: (timeout) => clearTimeout(timeout),
    },
    private readonly logError: (message: string, error: unknown) => void = (
      message,
      error
    ) => console.error(message, error)
  ) {}

  handleRequest(
    request: NativeNavigationPolicyRequest,
    handler: NavigationPolicyHandler | undefined
  ): void {
    this.pendingNavigationRequests.add(request.navigationId);
    this.pendingNavigationTimeouts.set(
      request.navigationId,
      this.timerApi.setTimeout(() => {
        if (this.resolveNativeFallbackOnTimeout) {
          this.resolve(request.navigationId, true);
        } else {
          this.clearPending(request.navigationId);
        }
      }, this.timeoutMs)
    );

    if (!handler) {
      this.resolve(request.navigationId, true);
      return;
    }

    let decision: boolean | Promise<boolean>;
    try {
      decision = handler(
        Object.freeze({
          url: request.url,
        })
      );
    } catch (error) {
      this.logError('Failed to resolve navigation request', error);
      this.resolve(request.navigationId, true);
      return;
    }

    Promise.resolve(decision)
      .then((shouldStart) => {
        this.resolve(
          request.navigationId,
          typeof shouldStart === 'boolean' ? shouldStart : true
        );
      })
      .catch((error) => {
        this.logError('Failed to resolve navigation request', error);
        this.resolve(request.navigationId, true);
      });
  }

  resolve(navigationId: string, allow: boolean): boolean {
    if (!this.clearPending(navigationId)) {
      return false;
    }

    this.resolveNativeDecision(navigationId, allow);
    return true;
  }

  private clearPending(navigationId: string): boolean {
    if (!this.pendingNavigationRequests.delete(navigationId)) {
      return false;
    }

    const timeout = this.pendingNavigationTimeouts.get(navigationId);
    if (timeout != null) {
      this.timerApi.clearTimeout(timeout);
      this.pendingNavigationTimeouts.delete(navigationId);
    }
    return true;
  }

  allowAllPending(): void {
    for (const navigationId of [...this.pendingNavigationRequests]) {
      this.resolve(navigationId, true);
    }
  }

  pendingCount(): number {
    return this.pendingNavigationRequests.size;
  }
}
