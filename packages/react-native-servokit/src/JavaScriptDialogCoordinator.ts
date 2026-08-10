export type JavaScriptDialogKind = 'alert' | 'confirm' | 'prompt';

export type JavaScriptDialogDismissedEvent = Readonly<{
  dialogId: string;
}>;

export type JavaScriptDialogRequest = Readonly<{
  dialogId: string;
  kind: JavaScriptDialogKind;
  message: string;
  defaultValue: string | null;
  confirm(promptValue?: string): void;
  dismiss(): void;
}>;

export type JavaScriptDialogHandler = (request: JavaScriptDialogRequest) => void;

export type NativeJavaScriptDialogRequest = Readonly<{
  dialogId: string;
  kind: string;
  message: string;
  defaultValue: string | null;
}>;

function normalizeDialogKind(kind: string): JavaScriptDialogKind {
  return kind === 'confirm' || kind === 'prompt' ? kind : 'alert';
}

export class JavaScriptDialogCoordinator {
  private readonly pendingDialogIds = new Set<string>();

  constructor(
    private readonly resolveNativeDialog: (
      dialogId: string,
      confirmed: boolean,
      promptValue: string | null
    ) => void,
    private readonly logError: (message: string, error: unknown) => void = (
      message,
      error
    ) => console.error(message, error)
  ) {}

  handleRequest(
    nativeRequest: NativeJavaScriptDialogRequest,
    handler: JavaScriptDialogHandler | undefined
  ): void {
    const kind = normalizeDialogKind(nativeRequest.kind);
    this.pendingDialogIds.add(nativeRequest.dialogId);

    if (!handler) {
      this.dismiss(nativeRequest.dialogId);
      return;
    }

    const request: JavaScriptDialogRequest = Object.freeze({
      dialogId: nativeRequest.dialogId,
      kind,
      message: nativeRequest.message,
      defaultValue: nativeRequest.defaultValue,
      confirm: (promptValue?: string) => {
        this.resolve(
          nativeRequest.dialogId,
          true,
          kind === 'prompt'
            ? (promptValue ?? nativeRequest.defaultValue ?? '')
            : (promptValue ?? null)
        );
      },
      dismiss: () => {
        this.dismiss(nativeRequest.dialogId);
      },
    });

    try {
      handler(request);
    } catch (error) {
      this.logError('Failed to handle JavaScript dialog request', error);
      this.dismiss(nativeRequest.dialogId);
      throw error;
    }
  }

  resolve(
    dialogId: string,
    confirmed: boolean,
    promptValue: string | null = null
  ): boolean {
    if (!this.pendingDialogIds.has(dialogId)) {
      return false;
    }

    this.pendingDialogIds.delete(dialogId);
    this.resolveNativeDialog(dialogId, confirmed, promptValue);
    return true;
  }

  dismiss(dialogId: string): boolean {
    return this.resolve(dialogId, false, null);
  }

  handleDismissed(dialogId: string): void {
    this.pendingDialogIds.delete(dialogId);
  }

  dismissAllPending(): void {
    for (const dialogId of [...this.pendingDialogIds]) {
      this.dismiss(dialogId);
    }
  }

  pendingCount(): number {
    return this.pendingDialogIds.size;
  }
}
