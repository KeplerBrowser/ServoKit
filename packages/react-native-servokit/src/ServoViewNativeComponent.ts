import {
  codegenNativeCommands,
  codegenNativeComponent,
  type CodegenTypes,
  type HostComponent,
  type ViewProps,
} from 'react-native';
import type * as React from 'react';

type NativeServoViewUrlChangedEvent = Readonly<{
  url: string;
}>;

type NativeServoViewPageTitleChangedEvent = Readonly<{
  title: string | null;
}>;

type NativeServoViewStatusTextChangedEvent = Readonly<{
  status: string | null;
}>;

type NativeServoViewLoadStatusChangedEvent = Readonly<{
  status: string;
}>;

type NativeServoViewHistoryChangedEvent = Readonly<{
  entries: string[];
  current: CodegenTypes.Int32;
  canGoBack: boolean;
  canGoForward: boolean;
}>;

type NativeServoViewFocusChangedEvent = Readonly<{
  isFocused: boolean;
}>;

type NativeServoViewCursorChangedEvent = Readonly<{
  cursor: string;
}>;

type NativeServoViewFullscreenChangedEvent = Readonly<{
  isFullscreen: boolean;
}>;

type NativeServoViewClosedEvent = Readonly<{}>;

type NativeServoViewCrashedEvent = Readonly<{
  reason: string;
  backtrace: string | null;
}>;

type NativeServoViewErrorEvent = Readonly<{
  code: CodegenTypes.Int32;
  message: string;
}>;

type NativeServoViewJavaScriptEvaluationResultEvent = Readonly<{
  evaluationId: string;
  ok: boolean;
  valueJson: string | null;
  errorType: string | null;
}>;

type NativeServoViewControllerReadyEvent = Readonly<{
  controllerHandle: string;
}>;

type NativeServoViewShouldStartLoadWithRequestRequestedEvent = Readonly<{
  navigationId: string;
  url: string;
}>;

type NativeServoViewCreateNewWebViewRequestedEvent = Readonly<{
  parentWebViewId: string;
  parentUrl: string | null;
  targetUrl: string | null;
  windowFeatures: string | null;
  policy: string;
}>;

type NativeServoViewJavaScriptDialogRequestedEvent = Readonly<{
  dialogId: string;
  kind: string;
  message: string;
  defaultValue: string | null;
}>;

type NativeServoViewJavaScriptDialogDismissedEvent = Readonly<{
  dialogId: string;
}>;

type NativeServoViewContextMenuRequestedEvent = Readonly<{
  contextMenuId: string;
  elementJson: string;
  servoItemsJson: string;
}>;

type NativeServoViewContextMenuItemSelectedEvent = Readonly<{
  itemJson: string;
  elementJson: string;
}>;

export interface NativeServoViewProps extends ViewProps {
  url: string;
  useReactNativeJavaScriptDialogs?: boolean;
  useReactNativeContextMenus?: boolean;
  useReactNativeOnShouldStartLoadWithRequest?: boolean;
  onUrlChanged?: CodegenTypes.DirectEventHandler<NativeServoViewUrlChangedEvent>;
  onPageTitleChanged?: CodegenTypes.DirectEventHandler<NativeServoViewPageTitleChangedEvent>;
  onStatusTextChanged?: CodegenTypes.DirectEventHandler<NativeServoViewStatusTextChangedEvent>;
  onLoadStatusChanged?: CodegenTypes.DirectEventHandler<NativeServoViewLoadStatusChangedEvent>;
  onHistoryChanged?: CodegenTypes.DirectEventHandler<NativeServoViewHistoryChangedEvent>;
  onFocusChanged?: CodegenTypes.DirectEventHandler<NativeServoViewFocusChangedEvent>;
  onCursorChanged?: CodegenTypes.DirectEventHandler<NativeServoViewCursorChangedEvent>;
  onFullscreenChanged?: CodegenTypes.DirectEventHandler<NativeServoViewFullscreenChangedEvent>;
  onClosed?: CodegenTypes.DirectEventHandler<NativeServoViewClosedEvent>;
  onCrashed?: CodegenTypes.DirectEventHandler<NativeServoViewCrashedEvent>;
  onError?: CodegenTypes.DirectEventHandler<NativeServoViewErrorEvent>;
  onJavaScriptEvaluationResult?: CodegenTypes.DirectEventHandler<NativeServoViewJavaScriptEvaluationResultEvent>;
  onControllerReady?: CodegenTypes.DirectEventHandler<NativeServoViewControllerReadyEvent>;
  onShouldStartLoadWithRequestRequested?: CodegenTypes.DirectEventHandler<NativeServoViewShouldStartLoadWithRequestRequestedEvent>;
  onCreateNewWebViewRequested?: CodegenTypes.DirectEventHandler<NativeServoViewCreateNewWebViewRequestedEvent>;
  onJavaScriptDialogRequested?: CodegenTypes.DirectEventHandler<NativeServoViewJavaScriptDialogRequestedEvent>;
  onJavaScriptDialogDismissed?: CodegenTypes.DirectEventHandler<NativeServoViewJavaScriptDialogDismissedEvent>;
  onContextMenuRequested?: CodegenTypes.DirectEventHandler<NativeServoViewContextMenuRequestedEvent>;
  onContextMenuItemSelected?: CodegenTypes.DirectEventHandler<NativeServoViewContextMenuItemSelectedEvent>;
}

interface NativeServoViewCommands {
  sendControllerCommand: (
    viewRef: React.ElementRef<HostComponent<NativeServoViewProps>>,
    commandJson: string
  ) => void;
}

export const Commands = codegenNativeCommands<NativeServoViewCommands>({
  supportedCommands: ['sendControllerCommand'],
});

export default codegenNativeComponent<NativeServoViewProps>('ServoView');
