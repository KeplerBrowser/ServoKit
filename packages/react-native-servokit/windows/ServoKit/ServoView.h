#pragma once

#include "pch.h"

#ifdef RNW_NEW_ARCH
#include "codegen/react/components/ServoViewSpec/ServoView.g.h"

#include <servokit_desktop_private.h>
#include <unordered_map>
#endif

namespace winrt::ServoKit::implementation
{
void RegisterServoViewNativeComponent(
    winrt::Microsoft::ReactNative::IReactPackageBuilder const &packageBuilder) noexcept;

#ifdef RNW_NEW_ARCH
struct ServoViewComponentView : winrt::implements<
                                    ServoViewComponentView,
                                    winrt::Windows::Foundation::IInspectable>,
                                ServoKitCodegen::BaseServoView<ServoViewComponentView>
{
    ~ServoViewComponentView();

    winrt::Microsoft::UI::Composition::Visual CreateVisual(
        winrt::Microsoft::ReactNative::ComponentView const &view) noexcept override;
    void Initialize(winrt::Microsoft::ReactNative::ComponentView const &view) noexcept override;
    void UpdateProps(
        winrt::Microsoft::ReactNative::ComponentView const &view,
        winrt::com_ptr<ServoKitCodegen::ServoViewProps> const &newProps,
        winrt::com_ptr<ServoKitCodegen::ServoViewProps> const &oldProps) noexcept override;
    void UpdateLayoutMetrics(
        winrt::Microsoft::ReactNative::ComponentView const &view,
        winrt::Microsoft::ReactNative::LayoutMetrics const &newLayoutMetrics,
        winrt::Microsoft::ReactNative::LayoutMetrics const &oldLayoutMetrics) noexcept override;
    void HandleSendControllerCommandCommand(std::string commandJson) noexcept override;
    void PumpForToken(uint64_t token) noexcept;
    void SchedulePumpForToken(uint64_t token) noexcept;
    void ReceiveEventJson(
        uint64_t token,
        uint64_t attachmentGeneration,
        char const *eventJson) noexcept;

  private:
    static LRESULT CALLBACK WindowProc(HWND hwnd, UINT message, WPARAM wparam, LPARAM lparam);

    void SyncSurface(winrt::Microsoft::ReactNative::ComponentView const &view) noexcept;
    void DetachSurface() noexcept;
    void DestroyHost() noexcept;
    void HostWasConsumed() noexcept;
    bool CreateHostIfNeeded() noexcept;
    bool CreateChildWindowIfNeeded(HWND parentHwnd) noexcept;
    void DestroyChildWindow() noexcept;
    void DispatchInput(ServokitDesktopPrivateInput input) noexcept;
    void ReleaseKeyboardCharacters() noexcept;
    void DispatchKeyboardCharacter(
        std::string const &text, bool repeat,
        uint32_t action = SERVOKIT_DESKTOP_PRIVATE_PRESSED) noexcept;
    void DispatchKeyboardNamedInput(uint32_t action, uint32_t keyCode, bool repeat) noexcept;
    void DispatchImeCommit(std::string const &text) noexcept;
    ServokitDesktopPrivateStatus DispatchControllerCommandJson(std::string const &commandJson) noexcept;
    void ResolveNavigationRequest(std::string const &navigationId, bool allow) noexcept;
    void ResolveSimpleDialog(std::string const &dialogId, bool confirmed, std::optional<std::string> promptValue) noexcept;
    void DismissContextMenu(std::string const &contextMenuId) noexcept;
    void LoadUrlIfNeeded(bool allowDetached = false) noexcept;
    std::string ConsumeCharInput(WPARAM wparam) noexcept;
    ServokitDesktopPrivateViewport CurrentViewport() const noexcept;
    void DispatchServoEvent(winrt::Windows::Data::Json::JsonObject const &event) noexcept;

    winrt::Microsoft::ReactNative::ComponentView::LayoutMetricsChanged_revoker m_layoutMetricsChangedRevoker;
    winrt::Microsoft::ReactNative::IReactDispatcher m_uiDispatcher{nullptr};
    winrt::Microsoft::UI::Composition::SpriteVisual m_visual{nullptr};
    std::wstring m_windowClassName;
    HWND m_parentHwnd{nullptr};
    HWND m_childHwnd{nullptr};
    DWORD m_ownerThreadId{0};
    std::string m_url;
    std::string m_loadedUrl;
    ServokitDesktopPrivateHost *m_host{nullptr};
    uint64_t m_token{0};
    uint64_t m_attachmentGeneration{0};
    ServokitDesktopPrivateViewport m_viewport{};
    wchar_t m_pendingHighSurrogate{0};
    std::unordered_map<uint32_t, std::string> m_keyboardCharacters;
    bool m_spaceKeyDown{false};
    bool m_isPumping{false};
};
#endif
} // namespace winrt::ServoKit::implementation
