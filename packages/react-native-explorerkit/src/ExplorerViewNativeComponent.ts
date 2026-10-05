import {
  codegenNativeCommands,
  codegenNativeComponent,
  type CodegenTypes,
  type HostComponent,
  type ViewProps,
} from 'react-native';
import type * as React from 'react';

type NativeExplorerViewUrlChangedEvent = Readonly<{
  url: string;
}>;

type NativeExplorerViewPageTitleChangedEvent = Readonly<{
  title: string | null;
}>;

type NativeExplorerViewStatusTextChangedEvent = Readonly<{
  status: string | null;
}>;

type NativeExplorerViewLoadStatusChangedEvent = Readonly<{
  status: string;
}>;

type NativeExplorerViewHistoryChangedEvent = Readonly<{
  entries: string[];
  current: CodegenTypes.Int32;
  canGoBack: boolean;
  canGoForward: boolean;
}>;

type NativeExplorerViewFocusChangedEvent = Readonly<{
  isFocused: boolean;
}>;

type NativeExplorerViewCursorChangedEvent = Readonly<{
  cursor: string;
}>;

type NativeExplorerViewFullscreenChangedEvent = Readonly<{
  isFullscreen: boolean;
}>;

type NativeExplorerViewClosedEvent = Readonly<{}>;

type NativeExplorerViewCrashedEvent = Readonly<{
  reason: string;
  backtrace: string | null;
}>;

type NativeExplorerViewErrorEvent = Readonly<{
  code: CodegenTypes.Int32;
  message: string;
}>;

type NativeExplorerViewJavaScriptEvaluationResultEvent = Readonly<{
  evaluationId: string;
  ok: boolean;
  valueJson: string | null;
  errorType: string | null;
}>;

type NativeExplorerViewControllerReadyEvent = Readonly<{
  controllerHandle: string;
}>;

type NativeExplorerViewShouldStartLoadWithRequestRequestedEvent = Readonly<{
  navigationId: string;
  url: string;
}>;

type NativeExplorerViewCreateNewWebViewRequestedEvent = Readonly<{
  parentWebViewId: string;
  parentUrl: string | null;
  targetUrl: string | null;
  windowFeatures: string | null;
  policy: string;
}>;

type NativeExplorerViewJavaScriptDialogRequestedEvent = Readonly<{
  dialogId: string;
  kind: string;
  message: string;
  defaultValue: string | null;
}>;

type NativeExplorerViewJavaScriptDialogDismissedEvent = Readonly<{
  dialogId: string;
}>;

type NativeExplorerViewContextMenuRequestedEvent = Readonly<{
  contextMenuId: string;
  elementJson: string;
  servoItemsJson: string;
}>;

type NativeExplorerViewContextMenuItemSelectedEvent = Readonly<{
  itemJson: string;
  elementJson: string;
}>;

export interface NativeExplorerViewProps extends ViewProps {
  url: string;
  useReactNativeJavaScriptDialogs?: boolean;
  useReactNativeContextMenus?: boolean;
  useReactNativeOnShouldStartLoadWithRequest?: boolean;
  onUrlChanged?: CodegenTypes.DirectEventHandler<NativeExplorerViewUrlChangedEvent>;
  onPageTitleChanged?: CodegenTypes.DirectEventHandler<NativeExplorerViewPageTitleChangedEvent>;
  onStatusTextChanged?: CodegenTypes.DirectEventHandler<NativeExplorerViewStatusTextChangedEvent>;
  onLoadStatusChanged?: CodegenTypes.DirectEventHandler<NativeExplorerViewLoadStatusChangedEvent>;
  onHistoryChanged?: CodegenTypes.DirectEventHandler<NativeExplorerViewHistoryChangedEvent>;
  onFocusChanged?: CodegenTypes.DirectEventHandler<NativeExplorerViewFocusChangedEvent>;
  onCursorChanged?: CodegenTypes.DirectEventHandler<NativeExplorerViewCursorChangedEvent>;
  onFullscreenChanged?: CodegenTypes.DirectEventHandler<NativeExplorerViewFullscreenChangedEvent>;
  onClosed?: CodegenTypes.DirectEventHandler<NativeExplorerViewClosedEvent>;
  onCrashed?: CodegenTypes.DirectEventHandler<NativeExplorerViewCrashedEvent>;
  onError?: CodegenTypes.DirectEventHandler<NativeExplorerViewErrorEvent>;
  onJavaScriptEvaluationResult?: CodegenTypes.DirectEventHandler<NativeExplorerViewJavaScriptEvaluationResultEvent>;
  onControllerReady?: CodegenTypes.DirectEventHandler<NativeExplorerViewControllerReadyEvent>;
  onShouldStartLoadWithRequestRequested?: CodegenTypes.DirectEventHandler<NativeExplorerViewShouldStartLoadWithRequestRequestedEvent>;
  onCreateNewWebViewRequested?: CodegenTypes.DirectEventHandler<NativeExplorerViewCreateNewWebViewRequestedEvent>;
  onJavaScriptDialogRequested?: CodegenTypes.DirectEventHandler<NativeExplorerViewJavaScriptDialogRequestedEvent>;
  onJavaScriptDialogDismissed?: CodegenTypes.DirectEventHandler<NativeExplorerViewJavaScriptDialogDismissedEvent>;
  onContextMenuRequested?: CodegenTypes.DirectEventHandler<NativeExplorerViewContextMenuRequestedEvent>;
  onContextMenuItemSelected?: CodegenTypes.DirectEventHandler<NativeExplorerViewContextMenuItemSelectedEvent>;
}

interface NativeExplorerViewCommands {
  sendControllerCommand: (
    viewRef: React.ElementRef<HostComponent<NativeExplorerViewProps>>,
    commandJson: string
  ) => void;
}

export const Commands = codegenNativeCommands<NativeExplorerViewCommands>({
  supportedCommands: ['sendControllerCommand'],
});

export default codegenNativeComponent<NativeExplorerViewProps>('ExplorerView');
