#include "pch.h"

#include "ServoView.h"

#ifdef RNW_NEW_ARCH

using namespace winrt::Microsoft::ReactNative;
using namespace winrt::Microsoft::UI::Composition;
using namespace winrt::Windows::Data::Json;

namespace winrt::ServoKit::implementation
{
namespace
{
std::string JsonString(JsonObject const &object, wchar_t const *name)
{
    auto value = object.GetNamedValue(name, nullptr);
    return value && value.ValueType() == JsonValueType::String
        ? winrt::to_string(value.GetString())
        : std::string{};
}

std::optional<std::string> JsonOptionalString(JsonObject const &object, wchar_t const *name)
{
    auto value = object.GetNamedValue(name, nullptr);
    if (!value || value.ValueType() == JsonValueType::Null) {
        return std::nullopt;
    }
    if (value.ValueType() == JsonValueType::String) {
        return winrt::to_string(value.GetString());
    }
    return std::nullopt;
}

bool JsonBool(JsonObject const &object, wchar_t const *name)
{
    auto value = object.GetNamedValue(name, nullptr);
    return value && value.ValueType() == JsonValueType::Boolean && value.GetBoolean();
}

int32_t JsonInt32(JsonObject const &object, wchar_t const *name)
{
    auto value = object.GetNamedValue(name, nullptr);
    if (!value || value.ValueType() != JsonValueType::Number) {
        return 0;
    }
    auto number = value.GetNumber();
    return static_cast<int32_t>(
        std::clamp<double>(
            number,
            static_cast<double>(std::numeric_limits<int32_t>::min()),
            static_cast<double>(std::numeric_limits<int32_t>::max())));
}

std::vector<std::string> JsonStringArray(JsonObject const &object, wchar_t const *name)
{
    std::vector<std::string> values;
    auto value = object.GetNamedValue(name, nullptr);
    if (!value || value.ValueType() != JsonValueType::Array) {
        return values;
    }
    for (auto const &entry : value.GetArray()) {
        if (entry.ValueType() == JsonValueType::String) {
            values.push_back(winrt::to_string(entry.GetString()));
        }
    }
    return values;
}

std::string Utf8FromUtf16(std::wstring_view value) noexcept
{
    if (value.empty() || value.size() > static_cast<size_t>(std::numeric_limits<int>::max())) {
        return {};
    }
    auto required = WideCharToMultiByte(
        CP_UTF8,
        WC_ERR_INVALID_CHARS,
        value.data(),
        static_cast<int>(value.size()),
        nullptr,
        0,
        nullptr,
        nullptr);
    if (required <= 0) {
        return {};
    }
    std::string result(static_cast<size_t>(required), '\0');
    WideCharToMultiByte(
        CP_UTF8,
        WC_ERR_INVALID_CHARS,
        value.data(),
        static_cast<int>(value.size()),
        result.data(),
        required,
        nullptr,
        nullptr);
    return result;
}

std::string Utf8FromCodePoint(uint32_t codePoint) noexcept
{
    std::wstring value;
    if (codePoint < 0x20) {
        return {};
    }
    if (codePoint <= 0xD7FF || (codePoint >= 0xE000 && codePoint <= 0xFFFF)) {
        value.push_back(static_cast<wchar_t>(codePoint));
    } else if (codePoint <= 0x10FFFF) {
        codePoint -= 0x10000;
        value.push_back(static_cast<wchar_t>(0xD800 + (codePoint >> 10)));
        value.push_back(static_cast<wchar_t>(0xDC00 + (codePoint & 0x3FF)));
    }
    return Utf8FromUtf16(value);
}

bool NamedKeyForVirtualKey(WPARAM key, uint32_t &keyCode) noexcept
{
    switch (key) {
        case VK_BACK:
            keyCode = SERVOKIT_DESKTOP_PRIVATE_KEY_BACKSPACE;
            return true;
        case VK_DELETE:
            keyCode = SERVOKIT_DESKTOP_PRIVATE_KEY_DELETE;
            return true;
        case VK_RETURN:
            keyCode = SERVOKIT_DESKTOP_PRIVATE_KEY_ENTER;
            return true;
        case VK_TAB:
            keyCode = SERVOKIT_DESKTOP_PRIVATE_KEY_TAB;
            return true;
        case VK_ESCAPE:
            keyCode = SERVOKIT_DESKTOP_PRIVATE_KEY_ESCAPE;
            return true;
        case VK_SPACE:
            keyCode = SERVOKIT_DESKTOP_PRIVATE_KEY_SPACE;
            return true;
        case VK_LEFT:
            keyCode = SERVOKIT_DESKTOP_PRIVATE_KEY_ARROW_LEFT;
            return true;
        case VK_RIGHT:
            keyCode = SERVOKIT_DESKTOP_PRIVATE_KEY_ARROW_RIGHT;
            return true;
        case VK_UP:
            keyCode = SERVOKIT_DESKTOP_PRIVATE_KEY_ARROW_UP;
            return true;
        case VK_DOWN:
            keyCode = SERVOKIT_DESKTOP_PRIVATE_KEY_ARROW_DOWN;
            return true;
        default:
            return false;
    }
}

HWND ResolveParentHwnd(winrt::Microsoft::ReactNative::ComponentView const &view) noexcept
{
    auto context = view.ReactContext();
    if (context != nullptr) {
        auto topLevelHwnd = reinterpret_cast<HWND>(
            winrt::Microsoft::ReactNative::ReactCoreInjection::GetTopLevelWindowId(
                context.Properties()));
        // Clip RNW's island against the sibling render HWND above it. Parenting
        // the render HWND inside the island instead breaks RNW popup hosting.
        auto islandHwnd = topLevelHwnd == nullptr ? nullptr : FindWindowExW(
            topLevelHwnd, nullptr, L"Microsoft.UI.Content.DesktopChildSiteBridge", nullptr);
        if (islandHwnd != nullptr) {
            SetWindowLongPtrW(islandHwnd, GWL_STYLE,
                GetWindowLongPtrW(islandHwnd, GWL_STYLE) | WS_CLIPSIBLINGS);
        }
        return topLevelHwnd;
    }
    return nullptr;
}

void CALLBACK WakeCallback(void *context, uint64_t token) noexcept
{
    auto view = static_cast<ServoViewComponentView *>(context);
    if (view != nullptr) {
        view->SchedulePumpForToken(token);
    }
}

void CALLBACK EventCallback(
    void *context,
    uint64_t token,
    uint64_t attachmentGeneration,
    char const *eventJson) noexcept
{
    auto view = static_cast<ServoViewComponentView *>(context);
    if (view != nullptr) {
        view->ReceiveEventJson(token, attachmentGeneration, eventJson);
    }
}
} // namespace

void RegisterServoViewNativeComponent(IReactPackageBuilder const &packageBuilder) noexcept
{
    ServoKitCodegen::RegisterServoViewNativeComponent<ServoViewComponentView>(
        packageBuilder,
        [](winrt::Microsoft::ReactNative::Composition::IReactCompositionViewComponentBuilder const &builder) noexcept {
            builder.SetViewFeatures(
                winrt::Microsoft::ReactNative::Composition::ComponentViewFeatures::Default &
                ~(winrt::Microsoft::ReactNative::Composition::ComponentViewFeatures::NativeBorder |
                  winrt::Microsoft::ReactNative::Composition::ComponentViewFeatures::Background));
        });
}

ServoViewComponentView::~ServoViewComponentView()
{
    DestroyHost();
    DestroyChildWindow();
}

Visual ServoViewComponentView::CreateVisual(winrt::Microsoft::ReactNative::ComponentView const &view) noexcept
{
    auto compositor = view.as<winrt::Microsoft::ReactNative::Composition::ComponentView>().Compositor();
    m_visual = compositor.CreateSpriteVisual();
    return m_visual;
}

void ServoViewComponentView::Initialize(winrt::Microsoft::ReactNative::ComponentView const &view) noexcept
{
    m_ownerThreadId = GetCurrentThreadId();
    auto reactContext = view.ReactContext();
    if (reactContext != nullptr) {
        m_uiDispatcher = reactContext.UIDispatcher();
    }
    m_windowClassName =
        L"ServoKitReactNativeWindows_" + std::to_wstring(reinterpret_cast<uintptr_t>(this));
    m_layoutMetricsChangedRevoker = view.LayoutMetricsChanged(
        winrt::auto_revoke,
        [weakThis = get_weak()](
            winrt::Windows::Foundation::IInspectable const &sender,
            winrt::Microsoft::ReactNative::LayoutMetricsChangedArgs const &args) noexcept {
            if (auto strongThis = weakThis.get()) {
                auto layoutView = sender.try_as<winrt::Microsoft::ReactNative::ComponentView>();
                if (layoutView == nullptr) {
                    return;
                }
                auto visual = strongThis->m_visual;
                if (visual != nullptr) {
                    auto metrics = args.NewLayoutMetrics();
                    auto scale = metrics.PointScaleFactor;
                    visual.Size({
                        metrics.Frame.Width * scale,
                        metrics.Frame.Height * scale,
                    });
                    visual.Offset({
                        metrics.Frame.X * scale,
                        metrics.Frame.Y * scale,
                        0.0f,
                    });
                }
                strongThis->SyncSurface(layoutView);
            }
        });
    SyncSurface(view);
}

void ServoViewComponentView::UpdateProps(
    winrt::Microsoft::ReactNative::ComponentView const &view,
    winrt::com_ptr<ServoKitCodegen::ServoViewProps> const &newProps,
    winrt::com_ptr<ServoKitCodegen::ServoViewProps> const &oldProps) noexcept
{
    ServoKitCodegen::BaseServoView<ServoViewComponentView>::UpdateProps(
        view,
        newProps,
        oldProps);
    m_url = newProps ? newProps->url : std::string{};
    SyncSurface(view);
}

void ServoViewComponentView::UpdateLayoutMetrics(
    winrt::Microsoft::ReactNative::ComponentView const &view,
    winrt::Microsoft::ReactNative::LayoutMetrics const &,
    winrt::Microsoft::ReactNative::LayoutMetrics const &) noexcept
{
    SyncSurface(view);
}

void ServoViewComponentView::HandleSendControllerCommandCommand(
    std::string commandJson) noexcept
{
    DispatchControllerCommandJson(commandJson);
}

ServokitDesktopPrivateStatus ServoViewComponentView::DispatchControllerCommandJson(
    std::string const &commandJson) noexcept
{
    if (!CreateHostIfNeeded() || commandJson.empty()) {
        return SERVOKIT_DESKTOP_PRIVATE_INVALID_ARGUMENT;
    }
    auto status = servokit_desktop_private_dispatch_controller_command(
        m_host,
        m_token,
        reinterpret_cast<uint8_t const *>(commandJson.data()),
        commandJson.size());
    if (status == SERVOKIT_DESKTOP_PRIVATE_OK) {
        PumpForToken(m_token);
    } else if (
        status == SERVOKIT_DESKTOP_PRIVATE_STALE_TOKEN ||
        status == SERVOKIT_DESKTOP_PRIVATE_PANIC) {
        HostWasConsumed();
    }
    return status;
}

LRESULT CALLBACK ServoViewComponentView::WindowProc(
    HWND hwnd,
    UINT message,
    WPARAM wparam,
    LPARAM lparam)
{
    auto view = reinterpret_cast<ServoViewComponentView *>(
        GetWindowLongPtrW(hwnd, GWLP_USERDATA));
    switch (message) {
        case WM_NCCREATE: {
            auto create = reinterpret_cast<CREATESTRUCTW *>(lparam);
            SetWindowLongPtrW(
                hwnd,
                GWLP_USERDATA,
                reinterpret_cast<LONG_PTR>(create->lpCreateParams));
            return TRUE;
        }
        case WM_SETFOCUS:
        case WM_KILLFOCUS:
            if (view != nullptr) {
                ServokitDesktopPrivateInput input{};
                input.kind = SERVOKIT_DESKTOP_PRIVATE_INPUT_FOCUS;
                if (message == WM_KILLFOCUS) {
                    view->ReleaseKeyboardCharacters();
                    view->m_pendingHighSurrogate = 0;
                    view->m_spaceKeyDown = false;
                }
                input.is_focused = message == WM_SETFOCUS ? 1 : 0;
                view->DispatchInput(input);
            }
            break;
        case WM_MOUSEMOVE:
            if (view != nullptr) {
                ServokitDesktopPrivateInput input{};
                input.kind = SERVOKIT_DESKTOP_PRIVATE_INPUT_POINTER_MOVE;
                input.x = static_cast<float>(GET_X_LPARAM(lparam));
                input.y = static_cast<float>(GET_Y_LPARAM(lparam));
                view->DispatchInput(input);
                TRACKMOUSEEVENT track{};
                track.cbSize = sizeof(track);
                track.dwFlags = TME_LEAVE;
                track.hwndTrack = hwnd;
                TrackMouseEvent(&track);
            }
            break;
        case WM_MOUSELEAVE:
            if (view != nullptr) {
                ServokitDesktopPrivateInput input{};
                input.kind = SERVOKIT_DESKTOP_PRIVATE_INPUT_POINTER_LEAVE;
                view->DispatchInput(input);
            }
            break;
        case WM_LBUTTONDOWN:
        case WM_LBUTTONUP:
        case WM_RBUTTONDOWN:
        case WM_RBUTTONUP:
        case WM_MBUTTONDOWN:
        case WM_MBUTTONUP:
            if (view != nullptr) {
                SetFocus(hwnd);
                ServokitDesktopPrivateInput input{};
                input.kind = SERVOKIT_DESKTOP_PRIVATE_INPUT_POINTER_BUTTON;
                input.x = static_cast<float>(GET_X_LPARAM(lparam));
                input.y = static_cast<float>(GET_Y_LPARAM(lparam));
                input.action =
                    message == WM_LBUTTONDOWN || message == WM_RBUTTONDOWN || message == WM_MBUTTONDOWN
                    ? SERVOKIT_DESKTOP_PRIVATE_PRESSED
                    : SERVOKIT_DESKTOP_PRIVATE_RELEASED;
                input.button_kind =
                    message == WM_RBUTTONDOWN || message == WM_RBUTTONUP
                    ? SERVOKIT_DESKTOP_PRIVATE_BUTTON_SECONDARY
                    : message == WM_MBUTTONDOWN || message == WM_MBUTTONUP
                    ? SERVOKIT_DESKTOP_PRIVATE_BUTTON_MIDDLE
                    : SERVOKIT_DESKTOP_PRIVATE_BUTTON_PRIMARY;
                view->DispatchInput(input);
            }
            break;
        case WM_MOUSEWHEEL:
        case WM_MOUSEHWHEEL:
            if (view != nullptr) {
                POINT point{
                    GET_X_LPARAM(lparam),
                    GET_Y_LPARAM(lparam),
                };
                ScreenToClient(hwnd, &point);
                auto delta = static_cast<double>(GET_WHEEL_DELTA_WPARAM(wparam)) / WHEEL_DELTA;
                ServokitDesktopPrivateInput input{};
                input.kind = SERVOKIT_DESKTOP_PRIVATE_INPUT_POINTER_WHEEL;
                input.x = static_cast<float>(point.x);
                input.y = static_cast<float>(point.y);
                input.scroll_mode = SERVOKIT_DESKTOP_PRIVATE_SCROLL_LINES;
                if (message == WM_MOUSEHWHEEL) {
                    input.delta_x = delta;
                } else {
                    input.delta_y = -delta;
                }
                view->DispatchInput(input);
            }
            break;
        case WM_KEYDOWN:
        case WM_SYSKEYDOWN:
        case WM_KEYUP:
        case WM_SYSKEYUP:
            if (view != nullptr) {
                auto action = message == WM_KEYUP || message == WM_SYSKEYUP
                    ? SERVOKIT_DESKTOP_PRIVATE_RELEASED : SERVOKIT_DESKTOP_PRIVATE_PRESSED;
                auto scanCode = static_cast<uint32_t>((lparam >> 16) & 0x1FF);
                if (action == SERVOKIT_DESKTOP_PRIVATE_RELEASED) {
                    auto character = view->m_keyboardCharacters.find(scanCode);
                    if (character != view->m_keyboardCharacters.end()) {
                        auto text = std::move(character->second);
                        view->m_keyboardCharacters.erase(character);
                        view->DispatchKeyboardCharacter(text, false, action);
                        return 0;
                    }
                }
                uint32_t keyCode = 0;
                if (NamedKeyForVirtualKey(wparam, keyCode)) {
                    if (keyCode == SERVOKIT_DESKTOP_PRIVATE_KEY_SPACE) {
                        view->m_spaceKeyDown = action == SERVOKIT_DESKTOP_PRIVATE_PRESSED;
                    }
                    auto repeat = action == SERVOKIT_DESKTOP_PRIVATE_PRESSED && (lparam & (1LL << 30)) != 0;
                    view->DispatchKeyboardNamedInput(action, keyCode, repeat);
                    return 0;
                }
            }
            break;
        case WM_CHAR:
            if (view != nullptr) {
                // Servo represents the named Space key as a character already.
                if (wparam == L' ' && view->m_spaceKeyDown) {
                    return 0;
                }
                auto text = view->ConsumeCharInput(wparam);
                if (!text.empty()) {
                    auto repeat = (lparam & (1LL << 30)) != 0;
                    view->m_keyboardCharacters[static_cast<uint32_t>((lparam >> 16) & 0x1FF)] = text;
                    view->DispatchKeyboardCharacter(text, repeat);
                    return 0;
                }
            }
            break;
        case WM_IME_CHAR:
            if (view != nullptr) {
                auto text = view->ConsumeCharInput(wparam);
                if (!text.empty()) {
                    view->DispatchImeCommit(text);
                    return 0;
                }
            }
            break;
        case WM_UNICHAR:
            if (wparam == UNICODE_NOCHAR) {
                return TRUE;
            }
            if (view != nullptr) {
                auto text = Utf8FromCodePoint(static_cast<uint32_t>(wparam));
                if (!text.empty()) {
                    auto repeat = (lparam & (1LL << 30)) != 0;
                    view->DispatchKeyboardCharacter(text, repeat);
                    return 0;
                }
            }
            break;
    }
    return DefWindowProcW(hwnd, message, wparam, lparam);
}

bool ServoViewComponentView::CreateHostIfNeeded() noexcept
{
    if (m_host != nullptr) {
        return true;
    }
    if (m_ownerThreadId == 0) {
        m_ownerThreadId = GetCurrentThreadId();
    }
    ServokitDesktopPrivateCallbacks callbacks{};
    callbacks.context = this;
    callbacks.wake = WakeCallback;
    ServokitDesktopPrivateHost *host = nullptr;
    uint64_t token = 0;
    auto status = servokit_desktop_private_create(callbacks, &host, &token);
    if (status != SERVOKIT_DESKTOP_PRIVATE_OK) {
        return false;
    }
    m_host = host;
    m_token = token;
    return true;
}

bool ServoViewComponentView::CreateChildWindowIfNeeded(HWND parentHwnd) noexcept
{
    if (m_childHwnd != nullptr) {
        return true;
    }
    if (parentHwnd == nullptr) {
        return false;
    }
    WNDCLASSW windowClass{};
    windowClass.lpfnWndProc = ServoViewComponentView::WindowProc;
    windowClass.hInstance = GetModuleHandleW(nullptr);
    windowClass.lpszClassName = m_windowClassName.c_str();
    RegisterClassW(&windowClass);
    m_childHwnd = CreateWindowExW(
        0,
        m_windowClassName.c_str(),
        L"",
        WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS | WS_CLIPCHILDREN,
        0,
        0,
        1,
        1,
        parentHwnd,
        nullptr,
        windowClass.hInstance,
        this);
    return m_childHwnd != nullptr;
}

void ServoViewComponentView::DestroyChildWindow() noexcept
{
    if (m_childHwnd != nullptr) {
        DestroyWindow(m_childHwnd);
        m_childHwnd = nullptr;
    }
    if (!m_windowClassName.empty()) {
        UnregisterClassW(m_windowClassName.c_str(), GetModuleHandleW(nullptr));
    }
}

ServokitDesktopPrivateViewport ServoViewComponentView::CurrentViewport() const noexcept
{
    ServokitDesktopPrivateViewport viewport{};
    if (m_childHwnd == nullptr) {
        return viewport;
    }
    RECT rect{};
    GetClientRect(m_childHwnd, &rect);
    viewport.width = static_cast<uint32_t>(std::max<LONG>(0, rect.right - rect.left));
    viewport.height = static_cast<uint32_t>(std::max<LONG>(0, rect.bottom - rect.top));
    auto dpi = GetDpiForWindow(m_childHwnd);
    viewport.scale_factor = dpi == 0
        ? 1.0F
        : static_cast<float>(dpi) / static_cast<float>(USER_DEFAULT_SCREEN_DPI);
    return viewport;
}

void ServoViewComponentView::SyncSurface(winrt::Microsoft::ReactNative::ComponentView const &view) noexcept
{
    auto visual = m_visual;
    if (visual == nullptr) {
        return;
    }
    m_parentHwnd = ResolveParentHwnd(view);
    if (!CreateChildWindowIfNeeded(m_parentHwnd) || !CreateHostIfNeeded()) {
        return;
    }

    auto size = visual.Size();
    SetWindowPos(
        m_childHwnd,
        HWND_TOP,
        static_cast<int>(visual.Offset().x),
        static_cast<int>(visual.Offset().y),
        static_cast<int>(std::max(0.0f, size.x)),
        static_cast<int>(std::max(0.0f, size.y)),
        SWP_NOACTIVATE | SWP_SHOWWINDOW);

    auto viewport = CurrentViewport();
    if (viewport.width == 0 || viewport.height == 0) {
        DetachSurface();
        return;
    }

    if (m_attachmentGeneration == 0) {
        LoadUrlIfNeeded(true);
        if (m_host == nullptr) {
            return;
        }
        ServokitDesktopPrivateNativeSurface surface{};
        surface.window = m_childHwnd;
        surface.display = GetModuleHandleW(nullptr);
        uint64_t generation = 0;
        auto status = servokit_desktop_private_attach(
            m_host,
            m_token,
            surface,
            viewport,
            &generation);
        if (status == SERVOKIT_DESKTOP_PRIVATE_OK) {
            m_attachmentGeneration = generation;
            m_viewport = viewport;
            PumpForToken(m_token);
            LoadUrlIfNeeded();
        } else if (
            status == SERVOKIT_DESKTOP_PRIVATE_RUNTIME_ERROR ||
            status == SERVOKIT_DESKTOP_PRIVATE_STALE_TOKEN ||
            status == SERVOKIT_DESKTOP_PRIVATE_PANIC) {
            HostWasConsumed();
        }
        return;
    }

    LoadUrlIfNeeded();
    if (m_viewport.width == viewport.width &&
        m_viewport.height == viewport.height &&
        m_viewport.scale_factor == viewport.scale_factor) {
        return;
    }
    auto status = servokit_desktop_private_update_viewport(m_host, m_token, viewport);
    if (status == SERVOKIT_DESKTOP_PRIVATE_OK) {
        m_viewport = viewport;
        PumpForToken(m_token);
    } else if (
        status == SERVOKIT_DESKTOP_PRIVATE_RUNTIME_ERROR ||
        status == SERVOKIT_DESKTOP_PRIVATE_STALE_TOKEN ||
        status == SERVOKIT_DESKTOP_PRIVATE_PANIC) {
        HostWasConsumed();
    }
}

void ServoViewComponentView::DetachSurface() noexcept
{
    if (m_host == nullptr || m_attachmentGeneration == 0) {
        return;
    }
    ReleaseKeyboardCharacters();
    m_pendingHighSurrogate = 0;
    m_spaceKeyDown = false;
    auto status = servokit_desktop_private_detach(m_host, m_token);
    if (status == SERVOKIT_DESKTOP_PRIVATE_OK) {
        m_attachmentGeneration = 0;
    } else if (
        status == SERVOKIT_DESKTOP_PRIVATE_RUNTIME_ERROR ||
        status == SERVOKIT_DESKTOP_PRIVATE_STALE_TOKEN ||
        status == SERVOKIT_DESKTOP_PRIVATE_PANIC) {
        HostWasConsumed();
    }
}

void ServoViewComponentView::DestroyHost() noexcept
{
    if (m_host != nullptr && m_token != 0) {
        auto host = m_host;
        auto token = m_token;
        HostWasConsumed();
        servokit_desktop_private_destroy(host, token);
    }
}

void ServoViewComponentView::HostWasConsumed() noexcept
{
    m_keyboardCharacters.clear();
    m_pendingHighSurrogate = 0;
    m_spaceKeyDown = false;
    m_host = nullptr;
    m_token = 0;
    m_attachmentGeneration = 0;
    m_loadedUrl.clear();
}

void ServoViewComponentView::PumpForToken(uint64_t token) noexcept
{
    if (m_host == nullptr || token == 0 || token != m_token || m_isPumping) {
        return;
    }
    m_isPumping = true;
    auto status = servokit_desktop_private_pump(m_host, m_token);
    if (status == SERVOKIT_DESKTOP_PRIVATE_OK) {
        status = servokit_desktop_private_drain_events(
            m_host,
            m_token,
            EventCallback,
            this);
    }
    m_isPumping = false;
    if (
        status == SERVOKIT_DESKTOP_PRIVATE_STALE_TOKEN ||
        status == SERVOKIT_DESKTOP_PRIVATE_PANIC) {
        HostWasConsumed();
    }
}

void ServoViewComponentView::SchedulePumpForToken(uint64_t token) noexcept
{
    auto dispatcher = m_uiDispatcher;
    if (dispatcher == nullptr) {
        if (m_ownerThreadId == 0 || GetCurrentThreadId() == m_ownerThreadId) {
            PumpForToken(token);
        }
        return;
    }

    try {
        auto weakThis = get_weak();
        dispatcher.Post([weakThis, token]() noexcept {
            if (auto strongThis = weakThis.get()) {
                strongThis->PumpForToken(token);
            }
        });
    } catch (...) {
    }
}

void ServoViewComponentView::DispatchInput(ServokitDesktopPrivateInput input) noexcept
{
    if (m_host == nullptr || m_attachmentGeneration == 0) {
        return;
    }
    auto status = servokit_desktop_private_dispatch_input(m_host, m_token, input);
    if (status == SERVOKIT_DESKTOP_PRIVATE_OK) {
        PumpForToken(m_token);
    } else if (
        status == SERVOKIT_DESKTOP_PRIVATE_STALE_TOKEN ||
        status == SERVOKIT_DESKTOP_PRIVATE_PANIC) {
        HostWasConsumed();
    }
}

void ServoViewComponentView::ReleaseKeyboardCharacters() noexcept
{
    auto characters = std::move(m_keyboardCharacters);
    m_keyboardCharacters.clear();
    for (auto const &entry : characters) {
        DispatchKeyboardCharacter(entry.second, false, SERVOKIT_DESKTOP_PRIVATE_RELEASED);
    }
}

void ServoViewComponentView::DispatchKeyboardCharacter(
    std::string const &text,
    bool repeat,
    uint32_t action) noexcept
{
    if (text.empty()) {
        return;
    }
    ServokitDesktopPrivateInput input{};
    input.kind = SERVOKIT_DESKTOP_PRIVATE_INPUT_KEYBOARD;
    input.action = action;
    input.key_kind = SERVOKIT_DESKTOP_PRIVATE_KEY_CHARACTER;
    input.text = text.c_str();
    input.repeat = repeat ? 1 : 0;
    DispatchInput(input);
}

void ServoViewComponentView::DispatchKeyboardNamedInput(
    uint32_t action,
    uint32_t keyCode,
    bool repeat) noexcept
{
    ServokitDesktopPrivateInput input{};
    input.kind = SERVOKIT_DESKTOP_PRIVATE_INPUT_KEYBOARD;
    input.action = action;
    input.key_kind = SERVOKIT_DESKTOP_PRIVATE_KEY_NAMED;
    input.key_code = keyCode;
    input.repeat = repeat ? 1 : 0;
    DispatchInput(input);
}

void ServoViewComponentView::DispatchImeCommit(std::string const &text) noexcept
{
    if (text.empty()) {
        return;
    }
    ServokitDesktopPrivateInput input{};
    input.kind = SERVOKIT_DESKTOP_PRIVATE_INPUT_IME_COMMIT;
    input.text = text.c_str();
    DispatchInput(input);
}

void ServoViewComponentView::ResolveNavigationRequest(
    std::string const &navigationId,
    bool allow) noexcept
{
    if (navigationId.empty()) {
        return;
    }
    JsonObject command;
    command.Insert(L"version", JsonValue::CreateNumberValue(1));
    command.Insert(L"command", JsonValue::CreateStringValue(L"resolveNavigationRequest"));
    command.Insert(L"navigationId", JsonValue::CreateStringValue(winrt::to_hstring(navigationId)));
    command.Insert(L"allow", JsonValue::CreateBooleanValue(allow));
    DispatchControllerCommandJson(winrt::to_string(command.Stringify()));
}

void ServoViewComponentView::ResolveSimpleDialog(
    std::string const &dialogId,
    bool confirmed,
    std::optional<std::string> promptValue) noexcept
{
    if (dialogId.empty()) {
        return;
    }
    JsonObject command;
    command.Insert(L"version", JsonValue::CreateNumberValue(1));
    command.Insert(L"command", JsonValue::CreateStringValue(L"resolveSimpleDialog"));
    command.Insert(L"dialogId", JsonValue::CreateStringValue(winrt::to_hstring(dialogId)));
    command.Insert(L"confirmed", JsonValue::CreateBooleanValue(confirmed));
    if (promptValue) {
        command.Insert(L"promptValue", JsonValue::CreateStringValue(winrt::to_hstring(*promptValue)));
    } else {
        command.Insert(L"promptValue", JsonValue::CreateNullValue());
    }
    DispatchControllerCommandJson(winrt::to_string(command.Stringify()));
}

void ServoViewComponentView::DismissContextMenu(std::string const &contextMenuId) noexcept
{
    if (contextMenuId.empty()) {
        return;
    }
    JsonObject command;
    command.Insert(L"version", JsonValue::CreateNumberValue(1));
    command.Insert(L"command", JsonValue::CreateStringValue(L"dismissContextMenu"));
    command.Insert(L"contextMenuId", JsonValue::CreateStringValue(winrt::to_hstring(contextMenuId)));
    DispatchControllerCommandJson(winrt::to_string(command.Stringify()));
}

void ServoViewComponentView::LoadUrlIfNeeded(bool allowDetached) noexcept
{
    if ((!allowDetached && m_attachmentGeneration == 0) ||
        m_url.empty() ||
        m_loadedUrl == m_url ||
        !CreateHostIfNeeded()) {
        return;
    }
    JsonObject command;
    command.Insert(L"version", JsonValue::CreateNumberValue(1));
    command.Insert(L"command", JsonValue::CreateStringValue(L"loadUrl"));
    command.Insert(L"url", JsonValue::CreateStringValue(winrt::to_hstring(m_url)));
    auto commandJson = winrt::to_string(command.Stringify());
    if (DispatchControllerCommandJson(commandJson) == SERVOKIT_DESKTOP_PRIVATE_OK) {
        m_loadedUrl = m_url;
    }
}

std::string ServoViewComponentView::ConsumeCharInput(WPARAM wparam) noexcept
{
    auto codeUnit = static_cast<wchar_t>(wparam);
    if (codeUnit >= 0xD800 && codeUnit <= 0xDBFF) {
        m_pendingHighSurrogate = codeUnit;
        return {};
    }

    std::wstring text;
    if (codeUnit >= 0xDC00 && codeUnit <= 0xDFFF) {
        if (m_pendingHighSurrogate == 0) {
            return {};
        }
        text.push_back(m_pendingHighSurrogate);
        text.push_back(codeUnit);
        m_pendingHighSurrogate = 0;
    } else {
        m_pendingHighSurrogate = 0;
        if (codeUnit < 0x20) {
            return {};
        }
        text.push_back(codeUnit);
    }
    return Utf8FromUtf16(text);
}

void ServoViewComponentView::ReceiveEventJson(
    uint64_t token,
    uint64_t attachmentGeneration,
    char const *eventJson) noexcept
{
    if (eventJson == nullptr ||
        token != m_token ||
        (attachmentGeneration != 0 && attachmentGeneration != m_attachmentGeneration)) {
        return;
    }
    JsonObject event{nullptr};
    if (JsonObject::TryParse(winrt::to_hstring(eventJson), event)) {
        DispatchServoEvent(event);
    }
}

void ServoViewComponentView::DispatchServoEvent(JsonObject const &event) noexcept
{
    auto name = JsonString(event, L"name");
    auto payloadValue = event.GetNamedValue(L"payload", nullptr);
    if (!payloadValue || payloadValue.ValueType() != JsonValueType::Object) {
        return;
    }
    auto payload = payloadValue.GetObject();
    auto emitter = EventEmitter();
    if (name == "navigationRequested") {
        auto navigationId = JsonString(payload, L"navigationId");
        auto props = Props();
        if (emitter && props && props->useReactNativeOnShouldStartLoadWithRequest.value_or(false)) {
            ServoKitCodegen::ServoViewSpec_onShouldStartLoadWithRequestRequested value{};
            value.navigationId = navigationId;
            value.url = JsonString(payload, L"url");
            emitter->onShouldStartLoadWithRequestRequested(std::move(value));
        } else {
            ResolveNavigationRequest(navigationId, true);
        }
    } else if (name == "popupRequested") {
        if (!emitter) {
            return;
        }
        ServoKitCodegen::ServoViewSpec_onCreateNewWebViewRequested value{};
        value.parentWebViewId = JsonString(payload, L"parentWebViewId");
        value.parentUrl = JsonOptionalString(payload, L"parentUrl");
        value.targetUrl = JsonOptionalString(payload, L"targetUrl");
        value.windowFeatures = JsonOptionalString(payload, L"windowFeatures");
        value.policy = JsonString(payload, L"policy");
        emitter->onCreateNewWebViewRequested(std::move(value));
    } else if (name == "simpleDialogRequested") {
        auto dialogId = JsonString(payload, L"dialogId");
        auto props = Props();
        if (emitter && props && props->useReactNativeJavaScriptDialogs.value_or(false)) {
            ServoKitCodegen::ServoViewSpec_onJavaScriptDialogRequested value{};
            value.dialogId = dialogId;
            value.kind = JsonString(payload, L"kind");
            value.message = JsonString(payload, L"message");
            value.defaultValue = JsonOptionalString(payload, L"defaultValue");
            emitter->onJavaScriptDialogRequested(std::move(value));
        } else {
            ResolveSimpleDialog(dialogId, false, std::nullopt);
        }
    } else if (name == "contextMenuRequested") {
        DismissContextMenu(JsonString(payload, L"contextMenuId"));
    } else if (!emitter) {
        return;
    } else if (name == "urlChanged") {
        ServoKitCodegen::ServoViewSpec_onUrlChanged value{};
        value.url = JsonString(payload, L"url");
        m_loadedUrl = value.url;
        emitter->onUrlChanged(std::move(value));
    } else if (name == "pageTitleChanged") {
        ServoKitCodegen::ServoViewSpec_onPageTitleChanged value{};
        value.title = JsonOptionalString(payload, L"title");
        emitter->onPageTitleChanged(std::move(value));
    } else if (name == "statusTextChanged") {
        ServoKitCodegen::ServoViewSpec_onStatusTextChanged value{};
        value.status = JsonOptionalString(payload, L"status");
        emitter->onStatusTextChanged(std::move(value));
    } else if (name == "loadStatusChanged") {
        ServoKitCodegen::ServoViewSpec_onLoadStatusChanged value{};
        value.status = JsonString(payload, L"status");
        emitter->onLoadStatusChanged(std::move(value));
    } else if (name == "historyChanged") {
        ServoKitCodegen::ServoViewSpec_onHistoryChanged value{};
        value.entries = JsonStringArray(payload, L"entries");
        value.current = JsonInt32(payload, L"current");
        value.canGoBack = JsonBool(payload, L"canGoBack");
        value.canGoForward = JsonBool(payload, L"canGoForward");
        emitter->onHistoryChanged(std::move(value));
    } else if (name == "focusChanged") {
        ServoKitCodegen::ServoViewSpec_onFocusChanged value{};
        value.isFocused = JsonBool(payload, L"isFocused");
        emitter->onFocusChanged(std::move(value));
    } else if (name == "cursorChanged") {
        ServoKitCodegen::ServoViewSpec_onCursorChanged value{};
        value.cursor = JsonString(payload, L"cursor");
        emitter->onCursorChanged(std::move(value));
    } else if (name == "fullscreenChanged") {
        ServoKitCodegen::ServoViewSpec_onFullscreenChanged value{};
        value.isFullscreen = JsonBool(payload, L"isFullscreen");
        emitter->onFullscreenChanged(std::move(value));
    } else if (name == "closed") {
        ServoKitCodegen::ServoViewSpec_onClosed value{};
        emitter->onClosed(std::move(value));
    } else if (name == "crashed") {
        ServoKitCodegen::ServoViewSpec_onCrashed value{};
        value.reason = JsonString(payload, L"reason");
        value.backtrace = JsonOptionalString(payload, L"backtrace");
        emitter->onCrashed(std::move(value));
    } else if (name == "error") {
        ServoKitCodegen::ServoViewSpec_onError value{};
        value.code = JsonInt32(payload, L"code");
        value.message = JsonString(payload, L"message");
        emitter->onError(std::move(value));
    } else if (name == "javascriptEvaluationResult") {
        ServoKitCodegen::ServoViewSpec_onJavaScriptEvaluationResult value{};
        value.evaluationId = JsonString(payload, L"evaluationId");
        value.ok = JsonBool(payload, L"ok");
        value.valueJson = JsonOptionalString(payload, L"valueJson");
        value.errorType = JsonOptionalString(payload, L"errorType");
        emitter->onJavaScriptEvaluationResult(std::move(value));
    } else if (name == "simpleDialogDismissed") {
        ServoKitCodegen::ServoViewSpec_onJavaScriptDialogDismissed value{};
        value.dialogId = JsonString(payload, L"dialogId");
        emitter->onJavaScriptDialogDismissed(std::move(value));
    }
}
} // namespace winrt::ServoKit::implementation
#else
namespace winrt::ServoKit::implementation
{
void RegisterServoViewNativeComponent(
    winrt::Microsoft::ReactNative::IReactPackageBuilder const &) noexcept
{
}
} // namespace winrt::ServoKit::implementation
#endif
