import * as React from 'react';
import {
  ExplorerView,
  type ExplorerViewCrashedEvent,
  type ExplorerViewCreateNewWebViewRequestedEvent,
  type ExplorerViewErrorEvent,
  type ExplorerViewHandle,
  type ExplorerViewLoadStatusChangedEvent,
} from '../index';

void React;

export function ExplorerViewTypeSmokeTest() {
  return (
    <ExplorerView
      url="https://example.com"
      onUrlChanged={(event) => {
        console.log('url', event.nativeEvent.url);
      }}
      onPageTitleChanged={(event) => {
        console.log('title', event.nativeEvent.title);
      }}
      onLoadStatusChanged={(event) => {
        console.log('status', event.nativeEvent.status);
      }}
      onHistoryChanged={(event) => {
        console.log(
          'history',
          event.nativeEvent.entries,
          event.nativeEvent.current,
          event.nativeEvent.canGoBack
        );
      }}
      onFocusChanged={(event) => {
        console.log('focus', event.nativeEvent.isFocused);
      }}
      onCursorChanged={(event) => {
        console.log('cursor', event.nativeEvent.cursor);
      }}
      onFullscreenChanged={(event) => {
        console.log('fullscreen', event.nativeEvent.isFullscreen);
      }}
      onError={(event) => {
        console.log(
          'error',
          event.nativeEvent.code,
          event.nativeEvent.message
        );
      }}
      onCrashed={(event) => {
        console.log(
          'crashed',
          event.nativeEvent.reason,
          event.nativeEvent.backtrace
        );
      }}
      onShouldStartLoadWithRequest={async ({ url }) => {
        console.log('should start', url);
        return true;
      }}
      onCreateNewWebViewRequested={(event) => {
        const request: ExplorerViewCreateNewWebViewRequestedEvent = event.nativeEvent;
        console.log(
          'create new webview',
          request.parentWebViewId,
          request.parentUrl,
          request.targetUrl,
          request.windowFeatures,
          request.policy
        );
      }}
      onBeforeShowContextMenu={async ({ element, servoItems, show }) => {
        console.log(element.contextType, servoItems.length);
        show([{ label: 'Bookmark', action: 'bookmark-link' }]);
      }}
      onContextMenuItemSelected={async ({ item, element }) => {
        console.log(item.type, element.contextType);
      }}
    />
  );
}

type HasUrl<T> = 'url' extends keyof T ? true : false;
const omittedEventUrls: [
  HasUrl<ExplorerViewLoadStatusChangedEvent>,
  HasUrl<ExplorerViewCrashedEvent>,
  HasUrl<ExplorerViewErrorEvent>,
] = [false, false, false];
void omittedEventUrls;

export function ExplorerViewRefTypeSmokeTest() {
  const ref = React.useRef<ExplorerViewHandle>(null);

  function handlePress() {
    ref.current?.loadUrl('https://servo.org');
    ref.current?.reload();
    ref.current?.goBack();
    ref.current?.goForward();
    ref.current?.focus();
    ref.current?.blur();
    void ref.current?.evaluateJavaScript('document.title').then((value) => {
      console.log(value);
    });
  }

  void handlePress;

  return <ExplorerView ref={ref} url="https://servo.org" />;
}
