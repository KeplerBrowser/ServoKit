import { useRef, useState } from "react";
import { ArrowLeft, ArrowRight, CircleArrowRight, House, Recycle, RotateCw } from "lucide-react-native";
import {
  Alert,
  Keyboard,
  KeyboardAvoidingView,
  Platform,
  Pressable,
  StyleSheet,
  Text,
  TextInput,
  View,
} from "react-native";
import { SafeAreaProvider, SafeAreaView } from "react-native-safe-area-context";
import {
  ServoView,
  type ServoViewHandle,
  type ServoViewJavaScriptDialogRequest,
} from "react-native-servokit";

const homeUrl = "https://servo.org";
const activeColor = "#007aff";
const disabledColor = "#c7c7cc";

export default function App() {
  const servoRef = useRef<ServoViewHandle>(null);
  const [pageUrl, setPageUrl] = useState(homeUrl);
  const [addressBarText, setAddressBarText] = useState(homeUrl);
  const [loadStatus, setLoadStatus] = useState("Idle");
  const [canGoBack, setCanGoBack] = useState(false);
  const [canGoForward, setCanGoForward] = useState(false);
  const [servoKey, setServoKey] = useState(0);

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

  const handleJavaScriptDialog = (request: ServoViewJavaScriptDialogRequest) => {
    const title =
      request.kind === "alert"
        ? "Page alert"
        : request.kind === "confirm"
          ? "Page confirmation"
          : "Page prompt";

    if (request.kind === "prompt" && Platform.OS === "ios") {
      Alert.prompt(
        title,
        request.message,
        [
          { text: "Cancel", style: "cancel", onPress: () => request.dismiss() },
          { text: "OK", onPress: (value) => request.confirm(value) },
        ],
        "plain-text",
        request.defaultValue,
      );
      return;
    }

    Alert.alert(
      title,
      request.message,
      request.kind === "alert"
        ? [{ text: "OK", onPress: () => request.confirm() }]
        : [
            { text: "Cancel", style: "cancel", onPress: () => request.dismiss() },
            {
              text: "OK",
              onPress: () =>
                request.confirm(request.kind === "prompt" ? request.defaultValue : undefined),
            },
          ],
      { cancelable: false },
    );
  };

  return (
    <SafeAreaProvider style={styles.container}>
      <KeyboardAvoidingView
        behavior={Platform.OS === "ios" ? "padding" : undefined}
        style={styles.container}
      >
        <SafeAreaView edges={["top", "left", "right"]} style={styles.workspace}>
          <ServoView
            key={servoKey}
            ref={servoRef}
            style={styles.webview}
            url={pageUrl}
            onUrlChanged={(event) => {
              setPageUrl(event.nativeEvent.url);
              setAddressBarText(event.nativeEvent.url);
            }}
            onLoadStatusChanged={(event) => setLoadStatus(event.nativeEvent.status)}
            onHistoryChanged={(event) => {
              setCanGoBack(event.nativeEvent.canGoBack);
              setCanGoForward(event.nativeEvent.canGoForward);
            }}
            onJavaScriptDialog={handleJavaScriptDialog}
            onError={(event) =>
              console.warn("ServoView error", event.nativeEvent.code, event.nativeEvent.message)
            }
            onCrashed={(event) =>
              console.warn(
                "ServoView crashed",
                event.nativeEvent.reason,
                event.nativeEvent.backtrace,
              )
            }
          />
        </SafeAreaView>
        <SafeAreaView edges={["bottom", "left", "right"]} style={styles.bottomChrome}>
          <View style={styles.addressRail}>
            <TextInput
              accessibilityLabel="Address"
              value={addressBarText}
              onChangeText={setAddressBarText}
              onSubmitEditing={() => navigateTo(addressBarText)}
              autoCapitalize="none"
              autoCorrect={false}
              keyboardType="url"
              placeholder="Enter a URL"
              placeholderTextColor="#8e8e93"
              returnKeyType="go"
              selectTextOnFocus
              style={styles.addressInput}
            />
            <Text accessibilityLabel={`Load status: ${loadStatus}`} numberOfLines={1} style={styles.loadStatus}>
              {loadStatus}
            </Text>
            <Pressable
              accessibilityLabel="Go to address"
              accessibilityRole="button"
              style={({ pressed }) => [styles.iconButton, pressed && styles.buttonPressed]}
              onPress={() => navigateTo(addressBarText)}
            >
              <CircleArrowRight color={activeColor} size={21} strokeWidth={2.2} />
            </Pressable>
          </View>
          <View style={styles.navigationRow}>
            <Pressable
              accessibilityLabel="Go back"
              accessibilityRole="button"
              disabled={!canGoBack}
              style={({ pressed }) => [styles.iconButton, pressed && canGoBack && styles.buttonPressed]}
              onPress={() => servoRef.current?.goBack()}
            >
              <ArrowLeft color={canGoBack ? activeColor : disabledColor} size={24} strokeWidth={2.2} />
            </Pressable>
            <Pressable
              accessibilityLabel="Go forward"
              accessibilityRole="button"
              disabled={!canGoForward}
              style={({ pressed }) => [
                styles.iconButton,
                pressed && canGoForward && styles.buttonPressed,
              ]}
              onPress={() => servoRef.current?.goForward()}
            >
              <ArrowRight
                color={canGoForward ? activeColor : disabledColor}
                size={24}
                strokeWidth={2.2}
              />
            </Pressable>
            <Pressable
              accessibilityLabel="Home"
              accessibilityRole="button"
              style={({ pressed }) => [styles.iconButton, pressed && styles.buttonPressed]}
              onPress={() => navigateTo(homeUrl)}
            >
              <House color={activeColor} size={23} strokeWidth={2.2} />
            </Pressable>
            <Pressable
              accessibilityLabel="Reload"
              accessibilityRole="button"
              style={({ pressed }) => [styles.iconButton, pressed && styles.buttonPressed]}
              onPress={() => servoRef.current?.reload()}
            >
              <RotateCw color={activeColor} size={23} strokeWidth={2.2} />
            </Pressable>
            <Pressable
              accessibilityLabel="Recycle Servo"
              accessibilityRole="button"
              style={({ pressed }) => [styles.iconButton, pressed && styles.buttonPressed]}
              onPress={() => setServoKey((key) => key + 1)}
            >
              <Recycle color={activeColor} size={23} strokeWidth={2.2} />
            </Pressable>
          </View>
        </SafeAreaView>
      </KeyboardAvoidingView>
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
  navigationRow: {
    height: 44,
    flexDirection: "row",
    alignItems: "center",
    justifyContent: "space-around",
  },
  iconButton: {
    width: 44,
    height: 44,
    alignItems: "center",
    justifyContent: "center",
  },
  buttonPressed: {
    opacity: 0.55,
  },
});
