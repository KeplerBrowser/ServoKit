import {
  Keyboard,
  KeyboardAvoidingView,
  Modal,
  Platform,
  Pressable,
  ScrollView,
  StyleSheet,
  Text,
  TextInput,
  View,
} from "react-native";
import { SafeAreaProvider, SafeAreaView } from "react-native-safe-area-context";
import { useRef, useState } from "react";
import {
  type ContextMenuElementInformation,
  type ContextMenuItem,
  ServoView,
  type ServoViewCursor,
  type ServoViewJavaScriptDialogRequest,
  type ServoViewHandle,
} from "react-native-servokit";

const fixtureServerBaseUrl = "http://127.0.0.1:8481";
const smokeFixturesUrl = `${fixtureServerBaseUrl}/smoke/index.html`;
const titleFixtureUrl = `${fixtureServerBaseUrl}/smoke/title-change.html`;
const historyFixtureUrl = `${fixtureServerBaseUrl}/smoke/history-start.html`;
const reloadFixtureUrl = `${fixtureServerBaseUrl}/smoke/reload.html`;
const formFixtureUrl = `${fixtureServerBaseUrl}/smoke/form.html`;
const errorFixtureUrl = `${fixtureServerBaseUrl}/smoke/error.html`;
const policyFixtureUrl = `${fixtureServerBaseUrl}/smoke/policy.html`;
const homeUrl = "https://servo.org";
const initialUrl = Platform.OS === "ios" ? homeUrl : smokeFixturesUrl;
const imeDemoUrl = `${fixtureServerBaseUrl}/controls/ime-form.html`;
const dialogDemoUrl = `${fixtureServerBaseUrl}/controls/dialogs.html`;
const pickerDemoUrl = `${fixtureServerBaseUrl}/controls/pickers.html`;
const permissionDemoUrl = `${fixtureServerBaseUrl}/controls/permissions.html`;
const contextMenuDemoUrl = `${fixtureServerBaseUrl}/controls/context-menu-demo.html`;
const blockedDemoUrl = "https://example.com/blocked-by-policy";

const readinessFixtures = [
  { id: "fixture.smoke-index", label: "Smoke index", url: smokeFixturesUrl },
  { id: "fixture.title-event", label: "Title event", url: titleFixtureUrl },
  { id: "fixture.history-flow", label: "History flow", url: historyFixtureUrl },
  { id: "fixture.reload", label: "Reload fixture", url: reloadFixtureUrl },
  { id: "fixture.shared-form", label: "Shared form", url: formFixtureUrl },
  { id: "fixture.error", label: "Error fixture", url: errorFixtureUrl },
  { id: "fixture.policy", label: "Policy checks", url: policyFixtureUrl },
  { id: "fixture.deny-policy", label: "Deny policy", url: blockedDemoUrl },
  { id: "fixture.dialog", label: "Dialog demo", url: dialogDemoUrl },
] as const;

const androidDemos = [
  { id: "fixture.home", label: "servo.org", url: homeUrl },
  { id: "fixture.ime", label: "IME demo", url: imeDemoUrl },
  { id: "fixture.picker", label: "Picker demo", url: pickerDemoUrl },
  {
    id: "fixture.permission",
    label: "Permission demo",
    url: permissionDemoUrl,
  },
  {
    id: "fixture.context-menu",
    label: "Context menu demo",
    url: contextMenuDemoUrl,
  },
] as const;

const fixtureGroups = [
  { label: "Readiness", fixtures: readinessFixtures },
  { label: "Controls", fixtures: androidDemos },
] as const;

const panelTabs = [
  { id: "fixtures", label: "Fixtures" },
  { id: "actions", label: "Actions" },
  { id: "status", label: "Status" },
] as const;

type PanelId = (typeof panelTabs)[number]["id"];

type PendingDialogState = Readonly<{
  request: ServoViewJavaScriptDialogRequest;
  promptValue: string;
}>;

export default function App() {
  const servoRef = useRef<ServoViewHandle>(null);
  const [pageUrl, setPageUrl] = useState(initialUrl);
  const [addressBarText, setAddressBarText] = useState(initialUrl);
  const [title, setTitle] = useState<string | null>(null);
  const [statusText, setStatusText] = useState<string | null>(null);
  const [loadStatus, setLoadStatus] = useState<string>("Idle");
  const [canGoBack, setCanGoBack] = useState(false);
  const [canGoForward, setCanGoForward] = useState(false);
  const [isFocused, setIsFocused] = useState(false);
  const [cursor, setCursor] = useState<ServoViewCursor>("default");
  const [isFullscreen, setIsFullscreen] = useState(false);
  const [lastError, setLastError] = useState<string>("(none yet)");
  const [pendingDialog, setPendingDialog] = useState<PendingDialogState | null>(null);
  const [lastContextMenuSelection, setLastContextMenuSelection] = useState<string>("(none yet)");
  const [lastNavigationPolicyDecision, setLastNavigationPolicyDecision] =
    useState<string>("(none yet)");
  const [lastJavaScriptEvaluation, setLastJavaScriptEvaluation] = useState<string>("(none yet)");
  const [activePanel, setActivePanel] = useState<PanelId>("fixtures");
  const [toolsVisible, setToolsVisible] = useState(false);
  const [servoViewGeneration, setServoViewGeneration] = useState(0);
  const activeDialog = pendingDialog?.request ?? null;
  const diagnosticRows = [
    { id: "status.url", label: "URL", value: pageUrl },
    { id: "status.title", label: "Title", value: title ?? "(none yet)" },
    {
      id: "status.navigation-policy",
      label: "Navigation policy",
      value: lastNavigationPolicyDecision,
    },
    {
      id: "status.status-text",
      label: "Status text",
      value: statusText ?? "(none yet)",
    },
    {
      id: "status.focus",
      label: "Focus",
      value: isFocused ? "focused" : "blurred",
    },
    { id: "status.cursor", label: "Cursor", value: cursor },
    {
      id: "status.fullscreen",
      label: "Fullscreen",
      value: isFullscreen ? "on" : "off",
    },
    {
      id: "status.context-menu",
      label: "Context menu",
      value: lastContextMenuSelection,
    },
    {
      id: "status.javascript-evaluation",
      label: "JavaScript evaluation",
      value: lastJavaScriptEvaluation,
    },
    { id: "status.last-error", label: "Last error", value: lastError },
  ];

  const createInjectedContextMenuItems = (
    element: ContextMenuElementInformation,
  ): ContextMenuItem[] => {
    const items: ContextMenuItem[] = [];
    if (element.isLink) {
      items.push({ label: "Bookmark link", action: "bookmark-link" });
    }
    if (element.hasSelection || element.contextType === "text") {
      items.push({
        label: "Translate selection",
        action: "translate-selection",
      });
    }
    if (element.isImage) {
      items.push({
        label: "Reverse image search",
        action: "reverse-image-search",
      });
    }
    if (element.isEditableText) {
      items.push({ label: "Clear field", action: "clear-editable-field" });
    }
    if (items.length === 0) {
      items.push({
        label: "Inspect page context",
        action: "inspect-page-context",
      });
    }
    return items;
  };

  const navigateTo = (url: string) => {
    const trimmedUrl = url.trim();
    if (!trimmedUrl) {
      return;
    }

    Keyboard.dismiss();
    setAddressBarText(trimmedUrl);
    if (Platform.OS === "ios") {
      setPageUrl(trimmedUrl);
    }
    servoRef.current?.loadUrl(trimmedUrl);
  };

  const openFixture = (url: string) => {
    navigateTo(url);
    setToolsVisible(false);
  };

  const evaluatePageTitle = async () => {
    try {
      const valueJson = await servoRef.current?.evaluateJavaScript("document.title");
      setLastJavaScriptEvaluation(valueJson ?? "(no mounted webview)");
    } catch (error) {
      const message = error instanceof Error ? `${error.name}:${error.message}` : String(error);
      setLastJavaScriptEvaluation(`error ${message}`);
    }
  };

  const closeDialog = () => {
    if (!activeDialog) {
      return;
    }

    activeDialog.dismiss();
    setPendingDialog(null);
  };

  const confirmDialog = () => {
    if (!pendingDialog) {
      return;
    }

    pendingDialog.request.confirm(
      pendingDialog.request.kind === "prompt" ? pendingDialog.promptValue : undefined,
    );
    setPendingDialog(null);
  };

  return (
    <SafeAreaProvider style={styles.container}>
      <KeyboardAvoidingView
        behavior={Platform.OS === "ios" ? "padding" : undefined}
        style={styles.container}
      >
        <SafeAreaView edges={["top", "left", "right"]} style={styles.workspace}>
          <ServoView
            key={servoViewGeneration}
            ref={servoRef}
            testID="browser.webview"
            style={styles.webview}
            url={pageUrl}
            onUrlChanged={(event) => {
              console.log("url changed", event.nativeEvent.url);
              setPageUrl(event.nativeEvent.url);
              setAddressBarText(event.nativeEvent.url);
            }}
            onPageTitleChanged={(event) => {
              console.log("title changed", event.nativeEvent.title);
              setTitle(event.nativeEvent.title);
            }}
            onStatusTextChanged={(event) => {
              console.log("status text changed", event.nativeEvent.status);
              setStatusText(event.nativeEvent.status);
            }}
            onLoadStatusChanged={(event) => {
              console.log("load status", event.nativeEvent.status);
              setLoadStatus(event.nativeEvent.status);
            }}
            onHistoryChanged={(event) => {
              console.log("history changed", event.nativeEvent.entries, event.nativeEvent.current);
              setCanGoBack(event.nativeEvent.canGoBack);
              setCanGoForward(event.nativeEvent.canGoForward);
            }}
            onFocusChanged={(event) => {
              console.log("focus changed", event.nativeEvent.isFocused);
              setIsFocused(event.nativeEvent.isFocused);
            }}
            onCursorChanged={(event) => {
              console.log("cursor changed", event.nativeEvent.cursor);
              setCursor(event.nativeEvent.cursor);
            }}
            onFullscreenChanged={(event) => {
              console.log("fullscreen changed", event.nativeEvent.isFullscreen);
              setIsFullscreen(event.nativeEvent.isFullscreen);
            }}
            onError={(event) => {
              console.log("error", event.nativeEvent.code, event.nativeEvent.message);
              setLastError(`${event.nativeEvent.code} ${event.nativeEvent.message}`);
            }}
            onShouldStartLoadWithRequest={async ({ url }) => {
              const shouldStart =
                /^https?:/i.test(url) && !url.toLowerCase().includes("blocked-by-policy");
              setLastNavigationPolicyDecision(`${shouldStart ? "allow" : "deny"} ${url}`);
              return shouldStart;
            }}
            onCrashed={(event) => {
              console.log("crashed", event.nativeEvent.reason, event.nativeEvent.backtrace);
              setLastError(`crashed: ${event.nativeEvent.reason}`);
            }}
            onJavaScriptDialog={(request) => {
              setPendingDialog({
                request,
                promptValue: request.defaultValue ?? "",
              });
            }}
            onJavaScriptDialogDismissed={(event) => {
              setPendingDialog((current) =>
                current?.request.dialogId === event.dialogId ? null : current,
              );
            }}
            onBeforeShowContextMenu={async ({ element, servoItems, show }) => {
              console.log("context menu requested", element, servoItems);
              show(createInjectedContextMenuItems(element));
            }}
            onContextMenuItemSelected={async ({ item, element }) => {
              const label = item.type === "separator" ? "separator" : item.label;
              const action = item.type === "separator" ? "separator" : item.action;
              setLastContextMenuSelection(
                `${item.source ?? "servo"}:${action} on ${element.contextType} (${label})`,
              );
              console.log("context menu item selected", item, element);
            }}
          />
        </SafeAreaView>
        <SafeAreaView edges={["bottom", "left", "right"]} style={styles.bottomChrome}>
          <View style={styles.addressRail}>
            <TextInput
              testID="browser.address"
              accessibilityLabel="Address"
              value={addressBarText}
              onChangeText={setAddressBarText}
              onSubmitEditing={() => navigateTo(addressBarText)}
              autoCapitalize="none"
              autoCorrect={false}
              keyboardType="url"
              placeholder="Enter a URL"
              placeholderTextColor="#64748b"
              returnKeyType="go"
              selectTextOnFocus
              style={styles.addressInput}
            />
            <Text
              testID="browser.load-status"
              accessibilityLabel={`Load status: ${loadStatus}`}
              numberOfLines={1}
              style={styles.loadStatus}
            >
              {loadStatus}
            </Text>
            <Pressable
              testID="browser.go"
              accessibilityLabel="Go to address"
              accessibilityRole="button"
              style={({ pressed }) => [styles.goButton, pressed && styles.buttonPressed]}
              onPress={() => navigateTo(addressBarText)}
            >
              <Text style={styles.goButtonLabel}>Go</Text>
            </Pressable>
          </View>
          <View style={styles.navigationRow}>
            <Pressable
              testID="browser.back"
              accessibilityLabel="Go back"
              accessibilityRole="button"
              disabled={!canGoBack}
              style={({ pressed }) => [
                styles.navButton,
                !canGoBack && styles.navButtonDisabled,
                pressed && canGoBack && styles.buttonPressed,
              ]}
              onPress={() => servoRef.current?.goBack()}
            >
              <Text style={styles.navButtonLabel}>Back</Text>
            </Pressable>
            <Pressable
              testID="browser.forward"
              accessibilityLabel="Go forward"
              accessibilityRole="button"
              disabled={!canGoForward}
              style={({ pressed }) => [
                styles.navButton,
                !canGoForward && styles.navButtonDisabled,
                pressed && canGoForward && styles.buttonPressed,
              ]}
              onPress={() => servoRef.current?.goForward()}
            >
              <Text style={styles.navButtonLabel}>Forward</Text>
            </Pressable>
            <Pressable
              testID="browser.home"
              accessibilityLabel="Home"
              accessibilityRole="button"
              style={({ pressed }) => [styles.navButton, pressed && styles.buttonPressed]}
              onPress={() => navigateTo(homeUrl)}
            >
              <Text style={styles.navButtonLabel}>Home</Text>
            </Pressable>
            <Pressable
              testID="browser.reload"
              accessibilityLabel="Reload"
              accessibilityRole="button"
              style={({ pressed }) => [styles.navButton, pressed && styles.buttonPressed]}
              onPress={() => servoRef.current?.reload()}
            >
              <Text style={styles.navButtonLabel}>Reload</Text>
            </Pressable>
            <Pressable
              testID="browser.tools"
              accessibilityLabel="Tools"
              accessibilityRole="button"
              style={({ pressed }) => [styles.navButton, pressed && styles.buttonPressed]}
              onPress={() => setToolsVisible(true)}
            >
              <Text style={styles.navButtonLabel}>Tools</Text>
            </Pressable>
          </View>
        </SafeAreaView>
      </KeyboardAvoidingView>
      <Modal
        allowSwipeDismissal={Platform.OS === "ios"}
        animationType="slide"
        presentationStyle={Platform.OS === "ios" ? "pageSheet" : "fullScreen"}
        visible={toolsVisible}
        onRequestClose={() => setToolsVisible(false)}
      >
        <SafeAreaProvider style={styles.toolsProvider}>
          <SafeAreaView
            testID="tools.modal"
            edges={["top", "bottom", "left", "right"]}
            style={styles.toolsSheet}
          >
            <View style={styles.toolsHeader}>
              <Text style={styles.toolsTitle}>Tools</Text>
              <Pressable
                testID="tools.close"
                accessibilityLabel="Close tools"
                accessibilityRole="button"
                style={({ pressed }) => [styles.closeButton, pressed && styles.buttonPressed]}
                onPress={() => setToolsVisible(false)}
              >
                <Text style={styles.closeButtonLabel}>Close</Text>
              </Pressable>
            </View>
            <View accessibilityRole="tablist" style={styles.panelTabs}>
              {panelTabs.map(({ id, label }) => (
                <Pressable
                  key={id}
                  testID={`tools.tab.${id}`}
                  accessibilityRole="tab"
                  accessibilityState={{ selected: activePanel === id }}
                  style={({ pressed }) => [
                    styles.panelTab,
                    activePanel === id && styles.panelTabActive,
                    pressed && styles.buttonPressed,
                  ]}
                  onPress={() => setActivePanel(id)}
                >
                  <Text
                    style={[styles.panelTabLabel, activePanel === id && styles.panelTabLabelActive]}
                  >
                    {label}
                  </Text>
                </Pressable>
              ))}
            </View>
            {activePanel === "fixtures" ? (
              <ScrollView
                style={styles.panelBody}
                contentContainerStyle={styles.panelBodyContent}
                keyboardShouldPersistTaps="handled"
              >
                {fixtureGroups.map(({ label, fixtures }) => (
                  <View key={label} style={styles.fixtureSection}>
                    <Text style={styles.sectionTitle}>{label}</Text>
                    <View style={styles.fixtureGrid}>
                      {fixtures.map(({ id, label: fixtureLabel, url }) => (
                        <Pressable
                          key={url}
                          testID={id}
                          accessibilityLabel={fixtureLabel}
                          accessibilityRole="button"
                          style={({ pressed }) => [
                            styles.fixtureButton,
                            pressed && styles.buttonPressed,
                          ]}
                          onPress={() => openFixture(url)}
                        >
                          <Text style={styles.fixtureButtonLabel}>{fixtureLabel}</Text>
                        </Pressable>
                      ))}
                    </View>
                  </View>
                ))}
              </ScrollView>
            ) : null}
            {activePanel === "actions" ? (
              <View style={[styles.panelBody, styles.actionPanel]}>
                <Pressable
                  testID="action.focus"
                  accessibilityRole="button"
                  style={({ pressed }) => [styles.actionButton, pressed && styles.buttonPressed]}
                  onPress={() => servoRef.current?.focus()}
                >
                  <Text style={styles.actionButtonLabel}>Focus</Text>
                </Pressable>
                <Pressable
                  testID="action.blur"
                  accessibilityRole="button"
                  style={({ pressed }) => [styles.actionButton, pressed && styles.buttonPressed]}
                  onPress={() => servoRef.current?.blur()}
                >
                  <Text style={styles.actionButtonLabel}>Blur</Text>
                </Pressable>
                <Pressable
                  testID="action.evaluate-title"
                  accessibilityRole="button"
                  style={({ pressed }) => [styles.actionButton, pressed && styles.buttonPressed]}
                  onPress={evaluatePageTitle}
                >
                  <Text style={styles.actionButtonLabel}>Evaluate title</Text>
                </Pressable>
                <Pressable
                  testID="action.recycle-view"
                  accessibilityRole="button"
                  style={({ pressed }) => [styles.actionButton, pressed && styles.buttonPressed]}
                  onPress={() => {
                    setServoViewGeneration((current) => current + 1);
                    setToolsVisible(false);
                  }}
                >
                  <Text style={styles.actionButtonLabel}>Recycle view</Text>
                </Pressable>
              </View>
            ) : null}
            {activePanel === "status" ? (
              <ScrollView style={styles.panelBody} contentContainerStyle={styles.statusList}>
                {diagnosticRows.map(({ id, label, value }) => (
                  <View key={label} style={styles.statusRow}>
                    <Text testID={id} style={styles.statusValue}>
                      {label}: {value}
                    </Text>
                  </View>
                ))}
              </ScrollView>
            ) : null}
          </SafeAreaView>
        </SafeAreaProvider>
      </Modal>
      <Modal
        animationType="fade"
        transparent
        visible={activeDialog != null}
        onRequestClose={closeDialog}
      >
        <View testID="dialog.modal" style={styles.dialogBackdrop}>
          <View accessibilityRole="alert" style={styles.dialogCard}>
            <Text testID="dialog.title" style={styles.dialogTitle}>
              {activeDialog?.kind === "prompt"
                ? "JavaScript prompt"
                : activeDialog?.kind === "confirm"
                  ? "JavaScript confirm"
                  : "JavaScript alert"}
            </Text>
            <Text testID="dialog.message" style={styles.dialogMessage}>
              {activeDialog?.message}
            </Text>
            {activeDialog?.kind === "prompt" ? (
              <TextInput
                testID="dialog.prompt-input"
                accessibilityLabel="Prompt response"
                value={pendingDialog?.promptValue ?? ""}
                onChangeText={(promptValue) =>
                  setPendingDialog((current) => (current ? { ...current, promptValue } : current))
                }
                autoCapitalize="none"
                autoCorrect={false}
                style={styles.dialogInput}
              />
            ) : null}
            <View style={styles.dialogActions}>
              {activeDialog?.kind !== "alert" ? (
                <Pressable
                  testID="dialog.cancel"
                  accessibilityRole="button"
                  style={styles.dialogSecondaryButton}
                  onPress={closeDialog}
                >
                  <Text style={styles.dialogSecondaryButtonLabel}>Cancel</Text>
                </Pressable>
              ) : null}
              <Pressable
                testID="dialog.confirm"
                accessibilityRole="button"
                style={styles.dialogPrimaryButton}
                onPress={confirmDialog}
              >
                <Text style={styles.dialogPrimaryButtonLabel}>
                  {activeDialog?.kind === "alert" ? "OK" : "Confirm"}
                </Text>
              </Pressable>
            </View>
          </View>
        </View>
      </Modal>
    </SafeAreaProvider>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: "#ffffff",
  },
  workspace: {
    flex: 1,
    backgroundColor: "#ffffff",
  },
  navigationRow: {
    height: 44,
    flexDirection: "row",
    alignItems: "center",
    justifyContent: "space-around",
  },
  navButton: {
    flex: 1,
    height: 44,
    alignItems: "center",
    justifyContent: "center",
  },
  navButtonDisabled: {
    opacity: 0.3,
  },
  navButtonLabel: {
    color: "#007aff",
    fontSize: 12,
    fontWeight: "600",
  },
  buttonPressed: {
    opacity: 0.55,
  },
  addressRail: {
    height: 38,
    flexDirection: "row",
    alignItems: "center",
    borderRadius: 8,
    backgroundColor: "#e9e9eb",
    overflow: "hidden",
  },
  addressInput: {
    flex: 1,
    height: 38,
    minWidth: 0,
    color: "#1c1c1e",
    paddingHorizontal: 12,
    paddingVertical: 8,
    fontSize: 13,
  },
  loadStatus: {
    width: 68,
    color: "#636366",
    fontSize: 11,
    fontWeight: "600",
    textAlign: "right",
  },
  goButton: {
    width: 44,
    height: 38,
    alignItems: "center",
    justifyContent: "center",
  },
  goButtonLabel: {
    color: "#007aff",
    fontWeight: "600",
    fontSize: 13,
  },
  sectionTitle: {
    color: "#1c1c1e",
    fontWeight: "700",
    fontSize: 13,
  },
  webview: {
    flex: 1,
    backgroundColor: "#ffffff",
  },
  bottomChrome: {
    flexGrow: 0,
    gap: 2,
    backgroundColor: "#f7f7f8",
    borderTopWidth: 1,
    borderTopColor: "#d1d1d6",
    paddingHorizontal: 8,
    paddingTop: 6,
    paddingBottom: 4,
  },
  toolsProvider: {
    flex: 1,
    backgroundColor: "#f2f2f7",
  },
  toolsSheet: {
    flex: 1,
    backgroundColor: "#f2f2f7",
    paddingHorizontal: 16,
    paddingBottom: 12,
  },
  toolsHeader: {
    height: 56,
    flexDirection: "row",
    alignItems: "center",
    justifyContent: "space-between",
  },
  toolsTitle: {
    color: "#1c1c1e",
    fontSize: 20,
    fontWeight: "700",
  },
  closeButton: {
    minWidth: 64,
    height: 44,
    alignItems: "center",
    justifyContent: "center",
  },
  closeButtonLabel: {
    color: "#007aff",
    fontSize: 14,
    fontWeight: "600",
  },
  panelTabs: {
    height: 40,
    flexDirection: "row",
    gap: 2,
    borderRadius: 8,
    backgroundColor: "#e3e3e8",
    padding: 2,
  },
  panelTab: {
    flex: 1,
    height: 36,
    borderRadius: 6,
    alignItems: "center",
    justifyContent: "center",
  },
  panelTabActive: {
    backgroundColor: "#ffffff",
  },
  panelTabLabel: {
    color: "#636366",
    fontSize: 13,
    fontWeight: "600",
  },
  panelTabLabelActive: {
    color: "#1c1c1e",
  },
  panelBody: {
    flex: 1,
    marginTop: 12,
  },
  panelBodyContent: {
    gap: 18,
    paddingBottom: 12,
  },
  fixtureSection: {
    gap: 8,
  },
  fixtureGrid: {
    flexDirection: "row",
    flexWrap: "wrap",
    gap: 7,
  },
  fixtureButton: {
    paddingHorizontal: 10,
    paddingVertical: 9,
    borderRadius: 7,
    backgroundColor: "#ffffff",
    borderWidth: 1,
    borderColor: "#d1d1d6",
  },
  fixtureButtonLabel: {
    color: "#1c1c1e",
    fontSize: 12,
    fontWeight: "600",
  },
  actionPanel: {
    flexDirection: "row",
    flexWrap: "wrap",
    alignContent: "flex-start",
    gap: 8,
  },
  actionButton: {
    minHeight: 44,
    paddingHorizontal: 12,
    borderRadius: 8,
    alignItems: "center",
    justifyContent: "center",
    backgroundColor: "#007aff",
  },
  actionButtonLabel: {
    color: "#ffffff",
    fontSize: 13,
    fontWeight: "700",
  },
  statusList: {
    gap: 8,
    paddingBottom: 12,
  },
  statusRow: {
    minHeight: 32,
    justifyContent: "center",
    borderBottomWidth: 1,
    borderBottomColor: "#d1d1d6",
    paddingBottom: 8,
  },
  statusValue: {
    color: "#1c1c1e",
    fontSize: 12,
  },
  dialogBackdrop: {
    flex: 1,
    backgroundColor: "rgba(15, 23, 42, 0.75)",
    alignItems: "center",
    justifyContent: "center",
    padding: 24,
  },
  dialogCard: {
    width: "100%",
    maxWidth: 360,
    backgroundColor: "#ffffff",
    borderRadius: 16,
    padding: 20,
    gap: 12,
  },
  dialogTitle: {
    fontSize: 18,
    fontWeight: "700",
    color: "#0f172a",
  },
  dialogMessage: {
    fontSize: 15,
    lineHeight: 22,
    color: "#334155",
  },
  dialogInput: {
    minHeight: 42,
    borderRadius: 10,
    borderWidth: 1,
    borderColor: "#cbd5e1",
    paddingHorizontal: 12,
    paddingVertical: 10,
    color: "#0f172a",
    backgroundColor: "#ffffff",
  },
  dialogActions: {
    flexDirection: "row",
    justifyContent: "flex-end",
    gap: 8,
  },
  dialogSecondaryButton: {
    paddingHorizontal: 14,
    paddingVertical: 10,
    borderRadius: 10,
    backgroundColor: "#e2e8f0",
  },
  dialogSecondaryButtonLabel: {
    color: "#0f172a",
    fontWeight: "600",
  },
  dialogPrimaryButton: {
    paddingHorizontal: 14,
    paddingVertical: 10,
    borderRadius: 10,
    backgroundColor: "#2563eb",
  },
  dialogPrimaryButtonLabel: {
    color: "#eff6ff",
    fontWeight: "600",
  },
});
