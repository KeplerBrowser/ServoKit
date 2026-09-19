import React, {
  forwardRef,
  useEffect,
  useImperativeHandle,
  useRef,
} from 'react';
import { findNodeHandle, Platform, UIManager, type CodegenTypes, type HostComponent } from 'react-native';

import NativeServoView, {
  Commands as NativeServoViewCommands,
  type NativeServoViewProps,
} from './ServoViewNativeComponent';
import {
  createJavaScriptEvaluationLifecycleError,
  JavaScriptEvaluationRegistry,
} from './JavaScriptEvaluationRegistry';
import {
  JavaScriptDialogCoordinator,
  type JavaScriptDialogDismissedEvent,
  type JavaScriptDialogKind,
  type JavaScriptDialogRequest,
} from './JavaScriptDialogCoordinator';
import { sendMountedControllerCommand } from './MountedControllerCommand';
import { NavigationPolicyCoordinator } from './NavigationPolicyCoordinator';

const NAVIGATION_DECISION_TIMEOUT_MS = 5000;
const servoViewManagerConfig = UIManager.getViewManagerConfig('ServoView') as
  | {
      Commands?: {
        showContextMenu?: number;
      };
    }
  | undefined;
const SHOW_CONTEXT_MENU_COMMAND = servoViewManagerConfig?.Commands?.showContextMenu;

/** Current URL from Servo `WebViewDelegate::notify_url_changed` or `WKWebView.URL` on iOS. */
export type ServoViewUrlChangedEvent = Readonly<{
  url: string;
}>;

/** Page title from Servo `WebViewDelegate::notify_page_title_changed` or `WKWebView.title` on iOS. */
export type ServoViewPageTitleChangedEvent = Readonly<{
  title: string | null;
}>;

/** Candidate URL from Servo `WebViewDelegate::request_navigation` or iOS `WKNavigationDelegate`. */
export type ServoViewShouldStartLoadRequest = Readonly<{
  url: string;
}>;

/**
 * React Native adapter event mapped from Servo
 * `WebViewDelegate::request_create_new` / `CreateNewWebViewRequest`.
 */
export type ServoViewCreateNewWebViewRequestedEvent = Readonly<{
  parentWebViewId: string;
  parentUrl: string | null;
  targetUrl: string | null;
  windowFeatures: string | null;
  policy: string;
}>;

/** Servo-backed status text from `WebViewDelegate::notify_status_text_changed`. */
export type ServoViewStatusTextChangedEvent = Readonly<{
  status: string | null;
}>;

/** Shared load states derived from Servo `LoadStatus` or iOS `WKNavigationDelegate`. */
export type ServoViewLoadStatus = 'Started' | 'HeadParsed' | 'Complete';

/** Load-state notification from Servo on Android or the WKWebView adapter on iOS. */
export type ServoViewLoadStatusChangedEvent = Readonly<{
  status: ServoViewLoadStatus;
}>;

/** History state from Servo `WebViewDelegate::notify_history_changed` or `WKBackForwardList`. */
export type ServoViewHistoryChangedEvent = Readonly<{
  entries: ReadonlyArray<string>;
  current: CodegenTypes.Int32;
  canGoBack: boolean;
  canGoForward: boolean;
}>;

/** Focus state from Servo `WebViewDelegate::notify_focus_changed` or WKWebView responder state. */
export type ServoViewFocusChangedEvent = Readonly<{
  isFocused: boolean;
}>;

/** Servo-backed CSS cursor name from `WebViewDelegate::notify_cursor_changed`. */
export type ServoViewCursor =
  | 'none'
  | 'default'
  | 'pointer'
  | 'context-menu'
  | 'help'
  | 'progress'
  | 'wait'
  | 'cell'
  | 'crosshair'
  | 'text'
  | 'vertical-text'
  | 'alias'
  | 'copy'
  | 'move'
  | 'no-drop'
  | 'not-allowed'
  | 'grab'
  | 'grabbing'
  | 'e-resize'
  | 'n-resize'
  | 'ne-resize'
  | 'nw-resize'
  | 's-resize'
  | 'se-resize'
  | 'sw-resize'
  | 'w-resize'
  | 'ew-resize'
  | 'ns-resize'
  | 'nesw-resize'
  | 'nwse-resize'
  | 'col-resize'
  | 'row-resize'
  | 'all-scroll'
  | 'zoom-in'
  | 'zoom-out';

/** Servo-backed cursor notification from `WebViewDelegate::notify_cursor_changed`. */
export type ServoViewCursorChangedEvent = Readonly<{
  cursor: ServoViewCursor;
}>;

/** Servo-backed fullscreen state from `WebViewDelegate::notify_fullscreen_state_changed`. */
export type ServoViewFullscreenChangedEvent = Readonly<{
  isFullscreen: boolean;
}>;

/** Servo-backed closure notification from `WebViewDelegate::notify_closed`. */
export type ServoViewClosedEvent = Readonly<{}>;

/** Servo-backed crash details from `WebViewDelegate::notify_crashed`. */
export type ServoViewCrashedEvent = Readonly<{
  reason: string;
  backtrace: string | null;
}>;

/** Package host error; this is adapter vocabulary, not a Servo delegate method. */
export type ServoViewErrorEvent = Readonly<{
  code: CodegenTypes.Int32;
  message: string;
}>;

/** Dialog kind mapped from Servo `SimpleDialog` on Android or `WKUIDelegate` on iOS. */
export type ServoViewJavaScriptDialogKind = JavaScriptDialogKind;
/** Package-owned notification that a pending platform dialog was dismissed. */
export type ServoViewJavaScriptDialogDismissedEvent = JavaScriptDialogDismissedEvent;
/** Package request wrapping Servo `SimpleDialog` on Android or a `WKUIDelegate` dialog on iOS. */
export type ServoViewJavaScriptDialogRequest = JavaScriptDialogRequest;

/** Android-only context category derived from Servo context-menu element information. */
export type ContextMenuElementContextType = 'text' | 'link' | 'image' | 'input' | 'default';
/** Android-only origin of a context-menu item: Servo or the React Native app. */
export type ContextMenuItemSource = 'servo' | 'app';

/** Android-only Servo context-menu target information normalized by the package adapter. */
export type ContextMenuElementInformation = Readonly<{
  isLink: boolean;
  isImage: boolean;
  isEditableText: boolean;
  hasSelection: boolean;
  linkUrl: string | null;
  imageUrl: string | null;
  contextType: ContextMenuElementContextType;
}>;

/** Android-only Servo or app-provided context-menu item. */
export type ContextMenuItem =
  | Readonly<{
      type?: 'item';
      source?: ContextMenuItemSource;
      label: string;
      action: string;
      enabled?: boolean;
    }>
  | Readonly<{
      type: 'separator';
      source?: ContextMenuItemSource;
    }>;

/** Android-only package hook for replacing Servo context-menu items before native presentation. */
export type ServoViewBeforeShowContextMenuEvent = Readonly<{
  element: ContextMenuElementInformation;
  servoItems: ReadonlyArray<ContextMenuItem>;
  show(items: ReadonlyArray<ContextMenuItem>): void;
}>;

/** Android-only package event for a selected native context-menu item. */
export type ServoViewContextMenuItemSelectedEvent = Readonly<{
  item: ContextMenuItem;
  element: ContextMenuElementInformation;
}>;

/**
 * Props for the ServoView component.
 *
 * Event handlers receive direct events from the platform's native engine adapter.
 * @see https://github.com/KeplerBrowser/ServoKit/blob/main/docs/host-control-capabilities.md
 */
export interface ServoViewProps
  extends Omit<
    NativeServoViewProps,
    | 'url'
    | 'useReactNativeJavaScriptDialogs'
    | 'useReactNativeContextMenus'
    | 'onUrlChanged'
    | 'onPageTitleChanged'
    | 'onStatusTextChanged'
    | 'onLoadStatusChanged'
    | 'onHistoryChanged'
    | 'onFocusChanged'
    | 'onCursorChanged'
    | 'onFullscreenChanged'
    | 'onClosed'
    | 'onCrashed'
    | 'onError'
    | 'onJavaScriptEvaluationResult'
    | 'onControllerReady'
    | 'useReactNativeOnShouldStartLoadWithRequest'
    | 'onShouldStartLoadWithRequestRequested'
    | 'onCreateNewWebViewRequested'
    | 'onJavaScriptDialogRequested'
    | 'onJavaScriptDialogDismissed'
    | 'onContextMenuRequested'
    | 'onContextMenuItemSelected'
  > {
  /** Initial and controlled URL; Android calls Servo `WebView::load`, while iOS calls `WKWebView.load(_:)`. */
  url: string;
  /** Fires when Servo `notify_url_changed` or observed `WKWebView.URL` changes. */
  onUrlChanged?: CodegenTypes.DirectEventHandler<ServoViewUrlChangedEvent>;
  /** Fires from Servo `notify_page_title_changed` or observed `WKWebView.title`. */
  onPageTitleChanged?: CodegenTypes.DirectEventHandler<ServoViewPageTitleChangedEvent>;
  /** Fires from Servo `notify_status_text_changed`. Android and Windows. */
  onStatusTextChanged?: CodegenTypes.DirectEventHandler<ServoViewStatusTextChangedEvent>;
  /** Fires from Servo `notify_load_status_changed` or iOS navigation delegate state. */
  onLoadStatusChanged?: CodegenTypes.DirectEventHandler<ServoViewLoadStatusChangedEvent>;
  /** Fires from Servo `notify_history_changed` or the WKWebView back-forward list. */
  onHistoryChanged?: CodegenTypes.DirectEventHandler<ServoViewHistoryChangedEvent>;
  /** Fires from Servo `notify_focus_changed` or WKWebView responder changes. */
  onFocusChanged?: CodegenTypes.DirectEventHandler<ServoViewFocusChangedEvent>;
  /** Fires from Servo `notify_cursor_changed`. Android and Windows. */
  onCursorChanged?: CodegenTypes.DirectEventHandler<ServoViewCursorChangedEvent>;
  /** Fires from Servo `notify_fullscreen_state_changed`. Android and Windows. */
  onFullscreenChanged?: CodegenTypes.DirectEventHandler<ServoViewFullscreenChangedEvent>;
  /** Fires from Servo `notify_closed`. Android and Windows. */
  onClosed?: CodegenTypes.DirectEventHandler<ServoViewClosedEvent>;
  /** Fires from Servo `notify_crashed`. Android and Windows. */
  onCrashed?: CodegenTypes.DirectEventHandler<ServoViewCrashedEvent>;
  /** Reports package host errors not represented by a specific Servo delegate event. Android and Windows. */
  onError?: CodegenTypes.DirectEventHandler<ServoViewErrorEvent>;
  /** Decides Servo `request_navigation` or WKNavigationDelegate policy. Android, Windows, and iOS; failures default to allow. */
  onShouldStartLoadWithRequest?: (
    request: ServoViewShouldStartLoadRequest
  ) => boolean | Promise<boolean>;
  /**
   * Servo-backed RN adapter event for create-new-webview requests. Android and Windows.
   * This prop name is not a Servo crate API.
   */
  onCreateNewWebViewRequested?: CodegenTypes.DirectEventHandler<ServoViewCreateNewWebViewRequestedEvent>;
  /** Handles Servo `SimpleDialog` or WKUIDelegate alert, confirm, and prompt requests. Android, Windows, and iOS. */
  onJavaScriptDialog?: (request: ServoViewJavaScriptDialogRequest) => void;
  /** Receives package-owned pending-dialog dismissal notifications. Android, Windows, and iOS. */
  onJavaScriptDialogDismissed?: (event: ServoViewJavaScriptDialogDismissedEvent) => void;
  /** Replaces Servo context-menu items before Android native presentation. Android only. */
  onBeforeShowContextMenu?: (
    event: ServoViewBeforeShowContextMenuEvent
  ) => Promise<void> | void;
  /** Handles selection of an Android native context-menu item. Android only. */
  onContextMenuItemSelected?: (
    event: ServoViewContextMenuItemSelectedEvent
  ) => Promise<void> | void;
}

/** Mounted commands supported by Servo-backed adapters and the iOS WKWebView adapter. */
export interface ServoViewHandle {
  /** Loads a URL with Servo `WebView::load` or `WKWebView.load(_:)` on iOS. */
  loadUrl(url: string): void;
  /** Calls Servo `WebView::reload` or `WKWebView.reload` on iOS. */
  reload(): void;
  /** Calls Servo `WebView::go_back(1)` or `WKWebView.goBack` on iOS. */
  goBack(): void;
  /** Calls Servo `WebView::go_forward(1)` or `WKWebView.goForward` on iOS. */
  goForward(): void;
  /** Calls Servo `WebView::focus` or `becomeFirstResponder` on iOS. */
  focus(): void;
  /** Calls Servo `WebView::blur` or `resignFirstResponder` on iOS. */
  blur(): void;
  /** Evaluates with Servo `WebView::evaluate_javascript` or `WKWebView.evaluateJavaScript`, returning tagged JSON. */
  evaluateJavaScript(script: string): Promise<string>;
}

type NativeRef = React.ElementRef<HostComponent<NativeServoViewProps>>;

type NativeContextMenuElementInformation = Readonly<{
  isLink: boolean;
  isImage: boolean;
  isEditableText: boolean;
  hasSelection: boolean;
  linkUrl: string | null;
  imageUrl: string | null;
  contextType: string;
}>;

type NativeContextMenuItem = Readonly<{
  type: string;
  source: string;
  label: string | null;
  action: string | null;
  enabled: boolean;
}>;

function createUnsupportedPlatformError(platform: string): Error {
  return new Error(
    `'react-native-servokit' currently supports Android, iOS, and Windows. Received unsupported platform '${platform}'.`
  );
}

function normalizeContextMenuElementInformation(
  element: NativeContextMenuElementInformation
): ContextMenuElementInformation {
  return Object.freeze({
    isLink: element.isLink,
    isImage: element.isImage,
    isEditableText: element.isEditableText,
    hasSelection: element.hasSelection,
    linkUrl: element.linkUrl,
    imageUrl: element.imageUrl,
    contextType: element.contextType as ContextMenuElementContextType,
  });
}

function normalizeContextMenuItem(item: NativeContextMenuItem): ContextMenuItem {
  if (item.type === 'separator') {
    return Object.freeze({
      type: 'separator',
      source: item.source as ContextMenuItemSource,
    });
  }

  return Object.freeze({
    type: 'item',
    source: item.source as ContextMenuItemSource,
    label: item.label ?? '',
    action: item.action ?? '',
    enabled: item.enabled,
  });
}

function serializeContextMenuItem(item: ContextMenuItem): Record<string, unknown> {
  if (item.type === 'separator') {
    return { type: 'separator', source: 'app' };
  }

  return {
    type: 'item',
    source: 'app',
    label: item.label,
    action: item.action,
    enabled: item.enabled ?? true,
  };
}

function parseContextMenuElementInformation(json: string): ContextMenuElementInformation {
  return normalizeContextMenuElementInformation(
    JSON.parse(json) as NativeContextMenuElementInformation
  );
}

function parseContextMenuItems(json: string): ReadonlyArray<ContextMenuItem> {
  const items = JSON.parse(json) as NativeContextMenuItem[];
  return Object.freeze(items.map((item) => normalizeContextMenuItem(item)));
}

function createControllerCommandPayload(
  command: string,
  payload: Record<string, unknown> = {}
): Record<string, unknown> {
  return {
    version: 1,
    command,
    ...payload,
  };
}

function createControllerCommandJson(
  command: string,
  payload: Record<string, unknown> = {}
): string {
  return JSON.stringify(createControllerCommandPayload(command, payload));
}

function dismissContextMenuCommandJson(contextMenuId: string): string {
  return createControllerCommandJson('dismissContextMenu', { contextMenuId });
}

/** Fabric web view backed by Servo on Android and Windows, and WKWebView on iOS. */
export const ServoView = forwardRef<ServoViewHandle, ServoViewProps>(
  function ServoView(props, ref) {
    const {
      onJavaScriptDialog,
      onJavaScriptDialogDismissed,
      onShouldStartLoadWithRequest,
      onBeforeShowContextMenu,
      onContextMenuItemSelected,
      ...nativeProps
    } = props;
    const nativeRef = useRef<NativeRef>(null);
    const javaScriptDialogCoordinatorRef = useRef<JavaScriptDialogCoordinator | null>(
      null
    );
    const navigationPolicyCoordinatorRef = useRef<NavigationPolicyCoordinator | null>(
      null
    );
    const pendingContextMenusRef = useRef<Set<string>>(new Set());
    const javaScriptEvaluationRegistryRef = useRef(
      new JavaScriptEvaluationRegistry()
    );

    const dispatchViewManagerCommand = (
      commandId: number | undefined,
      args: ReadonlyArray<unknown>
    ): boolean => {
      const nativeView = nativeRef.current;
      if (!nativeView) {
        return false;
      }

      const nativeViewTag = findNodeHandle(nativeView);
      if (nativeViewTag == null || commandId == null) {
        return false;
      }

      try {
        UIManager.dispatchViewManagerCommand(nativeViewTag, commandId, [...args]);
        return true;
      } catch {
        return false;
      }
    };

    const sendControllerCommand = (commandJson: string): boolean => {
      return sendMountedControllerCommand(
        nativeRef.current,
        commandJson,
        NativeServoViewCommands
      );
    };

    if (javaScriptDialogCoordinatorRef.current == null) {
      javaScriptDialogCoordinatorRef.current = new JavaScriptDialogCoordinator(
        (dialogId, confirmed, promptValue) => {
          sendControllerCommand(
            createControllerCommandJson('resolveSimpleDialog', {
              dialogId,
              confirmed,
              promptValue,
            })
          );
        }
      );
    }

    if (navigationPolicyCoordinatorRef.current == null) {
      navigationPolicyCoordinatorRef.current = new NavigationPolicyCoordinator(
        (navigationId, allow) => {
          sendControllerCommand(
            createControllerCommandJson('resolveNavigationRequest', {
              navigationId,
              allow,
            })
          );
        },
        NAVIGATION_DECISION_TIMEOUT_MS,
        Platform.OS === 'android' || Platform.OS === 'windows'
      );
    }

    useImperativeHandle(ref, () => ({
      loadUrl(url: string) {
        sendControllerCommand(createControllerCommandJson('loadUrl', { url }));
      },
      reload() {
        sendControllerCommand(createControllerCommandJson('reload'));
      },
      goBack() {
        sendControllerCommand(createControllerCommandJson('goBack'));
      },
      goForward() {
        sendControllerCommand(createControllerCommandJson('goForward'));
      },
      focus() {
        sendControllerCommand(createControllerCommandJson('focus'));
      },
      blur() {
        sendControllerCommand(createControllerCommandJson('blur'));
      },
      evaluateJavaScript(script: string) {
        const { evaluationId, promise } =
          javaScriptEvaluationRegistryRef.current.createPendingEvaluation();
        const sent = sendControllerCommand(
          createControllerCommandJson('evaluateJavaScript', {
            evaluationId,
            script,
          })
        );

        if (!sent) {
          javaScriptEvaluationRegistryRef.current.rejectPendingEvaluation(
            evaluationId,
            createJavaScriptEvaluationLifecycleError(
              'ServoView is not mounted, so JavaScript evaluation could not be dispatched.'
            )
          );
        }

        return promise;
      },
    }), []);

    useEffect(() => {
      return () => {
        javaScriptEvaluationRegistryRef.current.rejectAll(
          createJavaScriptEvaluationLifecycleError(
            'ServoView unmounted before JavaScript evaluation completed.'
          )
        );
        javaScriptDialogCoordinatorRef.current?.dismissAllPending();
        navigationPolicyCoordinatorRef.current?.allowAllPending();
      };
    }, []);

    if (
      Platform.OS !== 'android' &&
      Platform.OS !== 'ios' &&
      Platform.OS !== 'windows'
    ) {
      throw createUnsupportedPlatformError(Platform.OS);
    }

    const showContextMenu = (
      contextMenuId: string,
      items: ReadonlyArray<ContextMenuItem>
    ) => {
      if (!pendingContextMenusRef.current.has(contextMenuId)) {
        return;
      }

      pendingContextMenusRef.current.delete(contextMenuId);
      const shown = dispatchViewManagerCommand(SHOW_CONTEXT_MENU_COMMAND, [
        contextMenuId,
        items.map(serializeContextMenuItem),
      ]);
      if (!shown) {
        sendControllerCommand(dismissContextMenuCommandJson(contextMenuId));
      }
    };

    const handleNavigationPolicyRequest = (
      nativeEvent: Readonly<{ navigationId: string; url: string }>
    ) => {
      navigationPolicyCoordinatorRef.current?.handleRequest(
        nativeEvent,
        onShouldStartLoadWithRequest
      );
    };

    return (
      <NativeServoView
        ref={nativeRef}
        {...(nativeProps as NativeServoViewProps)}
        useReactNativeJavaScriptDialogs={Boolean(onJavaScriptDialog)}
        useReactNativeContextMenus={Boolean(onBeforeShowContextMenu)}
        useReactNativeOnShouldStartLoadWithRequest={Boolean(onShouldStartLoadWithRequest)}
        onJavaScriptEvaluationResult={(event) => {
          javaScriptEvaluationRegistryRef.current.settleEvaluation(event.nativeEvent);
        }}
        onShouldStartLoadWithRequestRequested={(event) => {
          handleNavigationPolicyRequest(event.nativeEvent);
        }}
        onJavaScriptDialogRequested={(event) => {
          javaScriptDialogCoordinatorRef.current?.handleRequest(
            event.nativeEvent,
            onJavaScriptDialog
          );
        }}
        onJavaScriptDialogDismissed={(event) => {
          javaScriptDialogCoordinatorRef.current?.handleDismissed(
            event.nativeEvent.dialogId
          );
          onJavaScriptDialogDismissed?.(event.nativeEvent);
        }}
        onContextMenuRequested={(event) => {
          const nativeEvent = event.nativeEvent;
          const element = parseContextMenuElementInformation(nativeEvent.elementJson);
          const servoItems = parseContextMenuItems(nativeEvent.servoItemsJson);
          pendingContextMenusRef.current.add(nativeEvent.contextMenuId);

          if (!onBeforeShowContextMenu) {
            showContextMenu(nativeEvent.contextMenuId, []);
            return;
          }

          let shown = false;
          const show = (items: ReadonlyArray<ContextMenuItem>) => {
            if (shown) {
              return;
            }

            shown = true;
            showContextMenu(nativeEvent.contextMenuId, items);
          };

          Promise.resolve(
            onBeforeShowContextMenu({
              element,
              servoItems,
              show,
            })
          )
            .then(() => {
              if (!shown) {
                show([]);
              }
            })
            .catch((error) => {
              if (!shown) {
                show([]);
              }
              console.error('Failed to prepare context menu', error);
            });
        }}
        onContextMenuItemSelected={(event) => {
          const nativeEvent = event.nativeEvent;
          const payload: ServoViewContextMenuItemSelectedEvent = Object.freeze({
            item: normalizeContextMenuItem(
              JSON.parse(nativeEvent.itemJson) as NativeContextMenuItem
            ),
            element: parseContextMenuElementInformation(nativeEvent.elementJson),
          });
          void Promise.resolve(onContextMenuItemSelected?.(payload)).catch((error) => {
            console.error('Failed to handle context menu item selection', error);
          });
        }}
      />
    );
  }
);
