
/*
 * This file is auto-generated from ServoViewNativeComponent spec file in flow / TypeScript.
 */
// clang-format off
#pragma once

#include <NativeModules.h>

#ifdef RNW_NEW_ARCH
#include <JSValueComposition.h>

#include <winrt/Microsoft.ReactNative.Composition.h>
#include <winrt/Microsoft.UI.Composition.h>
#endif // #ifdef RNW_NEW_ARCH

#ifdef RNW_NEW_ARCH

namespace ServoKitCodegen {

REACT_STRUCT(ServoViewProps)
struct ServoViewProps : winrt::implements<ServoViewProps, winrt::Microsoft::ReactNative::IComponentProps> {
  ServoViewProps(winrt::Microsoft::ReactNative::ViewProps props, const winrt::Microsoft::ReactNative::IComponentProps& cloneFrom)
    : ViewProps(props)
  {
     if (cloneFrom) {
       auto cloneFromProps = cloneFrom.as<ServoViewProps>();
       url = cloneFromProps->url;
       useReactNativeJavaScriptDialogs = cloneFromProps->useReactNativeJavaScriptDialogs;
       useReactNativeContextMenus = cloneFromProps->useReactNativeContextMenus;
       useReactNativeOnShouldStartLoadWithRequest = cloneFromProps->useReactNativeOnShouldStartLoadWithRequest;
       onUrlChanged = cloneFromProps->onUrlChanged;
       onPageTitleChanged = cloneFromProps->onPageTitleChanged;
       onStatusTextChanged = cloneFromProps->onStatusTextChanged;
       onLoadStatusChanged = cloneFromProps->onLoadStatusChanged;
       onHistoryChanged = cloneFromProps->onHistoryChanged;
       onFocusChanged = cloneFromProps->onFocusChanged;
       onCursorChanged = cloneFromProps->onCursorChanged;
       onFullscreenChanged = cloneFromProps->onFullscreenChanged;
       onClosed = cloneFromProps->onClosed;
       onCrashed = cloneFromProps->onCrashed;
       onError = cloneFromProps->onError;
       onJavaScriptEvaluationResult = cloneFromProps->onJavaScriptEvaluationResult;
       onControllerReady = cloneFromProps->onControllerReady;
       onShouldStartLoadWithRequestRequested = cloneFromProps->onShouldStartLoadWithRequestRequested;
       onCreateNewWebViewRequested = cloneFromProps->onCreateNewWebViewRequested;
       onJavaScriptDialogRequested = cloneFromProps->onJavaScriptDialogRequested;
       onJavaScriptDialogDismissed = cloneFromProps->onJavaScriptDialogDismissed;
       onContextMenuRequested = cloneFromProps->onContextMenuRequested;
       onContextMenuItemSelected = cloneFromProps->onContextMenuItemSelected;  
     }
  }

  void SetProp(uint32_t hash, winrt::hstring propName, winrt::Microsoft::ReactNative::IJSValueReader value) noexcept {
    winrt::Microsoft::ReactNative::ReadProp(hash, propName, value, *this);
  }

  REACT_FIELD(url)
  std::string url;

  REACT_FIELD(useReactNativeJavaScriptDialogs)
  std::optional<bool> useReactNativeJavaScriptDialogs{};

  REACT_FIELD(useReactNativeContextMenus)
  std::optional<bool> useReactNativeContextMenus{};

  REACT_FIELD(useReactNativeOnShouldStartLoadWithRequest)
  std::optional<bool> useReactNativeOnShouldStartLoadWithRequest{};

   // These fields can be used to determine if JS has registered for this event
  REACT_FIELD(onUrlChanged)
  bool onUrlChanged{false};

  REACT_FIELD(onPageTitleChanged)
  bool onPageTitleChanged{false};

  REACT_FIELD(onStatusTextChanged)
  bool onStatusTextChanged{false};

  REACT_FIELD(onLoadStatusChanged)
  bool onLoadStatusChanged{false};

  REACT_FIELD(onHistoryChanged)
  bool onHistoryChanged{false};

  REACT_FIELD(onFocusChanged)
  bool onFocusChanged{false};

  REACT_FIELD(onCursorChanged)
  bool onCursorChanged{false};

  REACT_FIELD(onFullscreenChanged)
  bool onFullscreenChanged{false};

  REACT_FIELD(onClosed)
  bool onClosed{false};

  REACT_FIELD(onCrashed)
  bool onCrashed{false};

  REACT_FIELD(onError)
  bool onError{false};

  REACT_FIELD(onJavaScriptEvaluationResult)
  bool onJavaScriptEvaluationResult{false};

  REACT_FIELD(onControllerReady)
  bool onControllerReady{false};

  REACT_FIELD(onShouldStartLoadWithRequestRequested)
  bool onShouldStartLoadWithRequestRequested{false};

  REACT_FIELD(onCreateNewWebViewRequested)
  bool onCreateNewWebViewRequested{false};

  REACT_FIELD(onJavaScriptDialogRequested)
  bool onJavaScriptDialogRequested{false};

  REACT_FIELD(onJavaScriptDialogDismissed)
  bool onJavaScriptDialogDismissed{false};

  REACT_FIELD(onContextMenuRequested)
  bool onContextMenuRequested{false};

  REACT_FIELD(onContextMenuItemSelected)
  bool onContextMenuItemSelected{false};

  const winrt::Microsoft::ReactNative::ViewProps ViewProps;
};

REACT_STRUCT(ServoViewSpec_onContextMenuItemSelected)
struct ServoViewSpec_onContextMenuItemSelected {
  REACT_FIELD(itemJson)
  std::string itemJson;

  REACT_FIELD(elementJson)
  std::string elementJson;
};

REACT_STRUCT(ServoViewSpec_onContextMenuRequested)
struct ServoViewSpec_onContextMenuRequested {
  REACT_FIELD(contextMenuId)
  std::string contextMenuId;

  REACT_FIELD(elementJson)
  std::string elementJson;

  REACT_FIELD(servoItemsJson)
  std::string servoItemsJson;
};

REACT_STRUCT(ServoViewSpec_onJavaScriptDialogDismissed)
struct ServoViewSpec_onJavaScriptDialogDismissed {
  REACT_FIELD(dialogId)
  std::string dialogId;
};

REACT_STRUCT(ServoViewSpec_onJavaScriptDialogRequested)
struct ServoViewSpec_onJavaScriptDialogRequested {
  REACT_FIELD(dialogId)
  std::string dialogId;

  REACT_FIELD(kind)
  std::string kind;

  REACT_FIELD(message)
  std::string message;

  REACT_FIELD(defaultValue)
  std::optional<std::string> defaultValue;
};

REACT_STRUCT(ServoViewSpec_onCreateNewWebViewRequested)
struct ServoViewSpec_onCreateNewWebViewRequested {
  REACT_FIELD(parentWebViewId)
  std::string parentWebViewId;

  REACT_FIELD(parentUrl)
  std::optional<std::string> parentUrl;

  REACT_FIELD(targetUrl)
  std::optional<std::string> targetUrl;

  REACT_FIELD(windowFeatures)
  std::optional<std::string> windowFeatures;

  REACT_FIELD(policy)
  std::string policy;
};

REACT_STRUCT(ServoViewSpec_onShouldStartLoadWithRequestRequested)
struct ServoViewSpec_onShouldStartLoadWithRequestRequested {
  REACT_FIELD(navigationId)
  std::string navigationId;

  REACT_FIELD(url)
  std::string url;
};

REACT_STRUCT(ServoViewSpec_onControllerReady)
struct ServoViewSpec_onControllerReady {
  REACT_FIELD(controllerHandle)
  std::string controllerHandle;
};

REACT_STRUCT(ServoViewSpec_onJavaScriptEvaluationResult)
struct ServoViewSpec_onJavaScriptEvaluationResult {
  REACT_FIELD(evaluationId)
  std::string evaluationId;

  REACT_FIELD(ok)
  bool ok{};

  REACT_FIELD(valueJson)
  std::optional<std::string> valueJson;

  REACT_FIELD(errorType)
  std::optional<std::string> errorType;
};

REACT_STRUCT(ServoViewSpec_onError)
struct ServoViewSpec_onError {
  REACT_FIELD(code)
  int32_t code{};

  REACT_FIELD(message)
  std::string message;
};

REACT_STRUCT(ServoViewSpec_onCrashed)
struct ServoViewSpec_onCrashed {
  REACT_FIELD(reason)
  std::string reason;

  REACT_FIELD(backtrace)
  std::optional<std::string> backtrace;
};

REACT_STRUCT(ServoViewSpec_onClosed)
struct ServoViewSpec_onClosed {
};

REACT_STRUCT(ServoViewSpec_onFullscreenChanged)
struct ServoViewSpec_onFullscreenChanged {
  REACT_FIELD(isFullscreen)
  bool isFullscreen{};
};

REACT_STRUCT(ServoViewSpec_onCursorChanged)
struct ServoViewSpec_onCursorChanged {
  REACT_FIELD(cursor)
  std::string cursor;
};

REACT_STRUCT(ServoViewSpec_onFocusChanged)
struct ServoViewSpec_onFocusChanged {
  REACT_FIELD(isFocused)
  bool isFocused{};
};

REACT_STRUCT(ServoViewSpec_onHistoryChanged)
struct ServoViewSpec_onHistoryChanged {
  REACT_FIELD(entries)
  std::vector<std::string> entries{};

  REACT_FIELD(current)
  int32_t current{};

  REACT_FIELD(canGoBack)
  bool canGoBack{};

  REACT_FIELD(canGoForward)
  bool canGoForward{};
};

REACT_STRUCT(ServoViewSpec_onLoadStatusChanged)
struct ServoViewSpec_onLoadStatusChanged {
  REACT_FIELD(status)
  std::string status;
};

REACT_STRUCT(ServoViewSpec_onStatusTextChanged)
struct ServoViewSpec_onStatusTextChanged {
  REACT_FIELD(status)
  std::optional<std::string> status;
};

REACT_STRUCT(ServoViewSpec_onPageTitleChanged)
struct ServoViewSpec_onPageTitleChanged {
  REACT_FIELD(title)
  std::optional<std::string> title;
};

REACT_STRUCT(ServoViewSpec_onUrlChanged)
struct ServoViewSpec_onUrlChanged {
  REACT_FIELD(url)
  std::string url;
};

struct ServoViewEventEmitter {
  ServoViewEventEmitter(const winrt::Microsoft::ReactNative::EventEmitter &eventEmitter)
      : m_eventEmitter(eventEmitter) {}

  using OnUrlChanged = ServoViewSpec_onUrlChanged;
  using OnPageTitleChanged = ServoViewSpec_onPageTitleChanged;
  using OnStatusTextChanged = ServoViewSpec_onStatusTextChanged;
  using OnLoadStatusChanged = ServoViewSpec_onLoadStatusChanged;
  using OnHistoryChanged = ServoViewSpec_onHistoryChanged;
  using OnFocusChanged = ServoViewSpec_onFocusChanged;
  using OnCursorChanged = ServoViewSpec_onCursorChanged;
  using OnFullscreenChanged = ServoViewSpec_onFullscreenChanged;
  using OnClosed = ServoViewSpec_onClosed;
  using OnCrashed = ServoViewSpec_onCrashed;
  using OnError = ServoViewSpec_onError;
  using OnJavaScriptEvaluationResult = ServoViewSpec_onJavaScriptEvaluationResult;
  using OnControllerReady = ServoViewSpec_onControllerReady;
  using OnShouldStartLoadWithRequestRequested = ServoViewSpec_onShouldStartLoadWithRequestRequested;
  using OnCreateNewWebViewRequested = ServoViewSpec_onCreateNewWebViewRequested;
  using OnJavaScriptDialogRequested = ServoViewSpec_onJavaScriptDialogRequested;
  using OnJavaScriptDialogDismissed = ServoViewSpec_onJavaScriptDialogDismissed;
  using OnContextMenuRequested = ServoViewSpec_onContextMenuRequested;
  using OnContextMenuItemSelected = ServoViewSpec_onContextMenuItemSelected;

  void onUrlChanged(OnUrlChanged &&value) const {
    m_eventEmitter.DispatchEvent(L"urlChanged", [value = std::move(value)](const winrt::Microsoft::ReactNative::IJSValueWriter writer) {
      winrt::Microsoft::ReactNative::WriteValue(writer, value);
    });
  }

  void onPageTitleChanged(OnPageTitleChanged &&value) const {
    m_eventEmitter.DispatchEvent(L"pageTitleChanged", [value = std::move(value)](const winrt::Microsoft::ReactNative::IJSValueWriter writer) {
      winrt::Microsoft::ReactNative::WriteValue(writer, value);
    });
  }

  void onStatusTextChanged(OnStatusTextChanged &&value) const {
    m_eventEmitter.DispatchEvent(L"statusTextChanged", [value = std::move(value)](const winrt::Microsoft::ReactNative::IJSValueWriter writer) {
      winrt::Microsoft::ReactNative::WriteValue(writer, value);
    });
  }

  void onLoadStatusChanged(OnLoadStatusChanged &&value) const {
    m_eventEmitter.DispatchEvent(L"loadStatusChanged", [value = std::move(value)](const winrt::Microsoft::ReactNative::IJSValueWriter writer) {
      winrt::Microsoft::ReactNative::WriteValue(writer, value);
    });
  }

  void onHistoryChanged(OnHistoryChanged &&value) const {
    m_eventEmitter.DispatchEvent(L"historyChanged", [value = std::move(value)](const winrt::Microsoft::ReactNative::IJSValueWriter writer) {
      winrt::Microsoft::ReactNative::WriteValue(writer, value);
    });
  }

  void onFocusChanged(OnFocusChanged &&value) const {
    m_eventEmitter.DispatchEvent(L"focusChanged", [value = std::move(value)](const winrt::Microsoft::ReactNative::IJSValueWriter writer) {
      winrt::Microsoft::ReactNative::WriteValue(writer, value);
    });
  }

  void onCursorChanged(OnCursorChanged &&value) const {
    m_eventEmitter.DispatchEvent(L"cursorChanged", [value = std::move(value)](const winrt::Microsoft::ReactNative::IJSValueWriter writer) {
      winrt::Microsoft::ReactNative::WriteValue(writer, value);
    });
  }

  void onFullscreenChanged(OnFullscreenChanged &&value) const {
    m_eventEmitter.DispatchEvent(L"fullscreenChanged", [value = std::move(value)](const winrt::Microsoft::ReactNative::IJSValueWriter writer) {
      winrt::Microsoft::ReactNative::WriteValue(writer, value);
    });
  }

  void onClosed(OnClosed &&value) const {
    m_eventEmitter.DispatchEvent(L"closed", [value = std::move(value)](const winrt::Microsoft::ReactNative::IJSValueWriter writer) {
      winrt::Microsoft::ReactNative::WriteValue(writer, value);
    });
  }

  void onCrashed(OnCrashed &&value) const {
    m_eventEmitter.DispatchEvent(L"crashed", [value = std::move(value)](const winrt::Microsoft::ReactNative::IJSValueWriter writer) {
      winrt::Microsoft::ReactNative::WriteValue(writer, value);
    });
  }

  void onError(OnError &&value) const {
    m_eventEmitter.DispatchEvent(L"error", [value = std::move(value)](const winrt::Microsoft::ReactNative::IJSValueWriter writer) {
      winrt::Microsoft::ReactNative::WriteValue(writer, value);
    });
  }

  void onJavaScriptEvaluationResult(OnJavaScriptEvaluationResult &&value) const {
    m_eventEmitter.DispatchEvent(L"javaScriptEvaluationResult", [value = std::move(value)](const winrt::Microsoft::ReactNative::IJSValueWriter writer) {
      winrt::Microsoft::ReactNative::WriteValue(writer, value);
    });
  }

  void onControllerReady(OnControllerReady &&value) const {
    m_eventEmitter.DispatchEvent(L"controllerReady", [value = std::move(value)](const winrt::Microsoft::ReactNative::IJSValueWriter writer) {
      winrt::Microsoft::ReactNative::WriteValue(writer, value);
    });
  }

  void onShouldStartLoadWithRequestRequested(OnShouldStartLoadWithRequestRequested &&value) const {
    m_eventEmitter.DispatchEvent(L"shouldStartLoadWithRequestRequested", [value = std::move(value)](const winrt::Microsoft::ReactNative::IJSValueWriter writer) {
      winrt::Microsoft::ReactNative::WriteValue(writer, value);
    });
  }

  void onCreateNewWebViewRequested(OnCreateNewWebViewRequested &&value) const {
    m_eventEmitter.DispatchEvent(L"createNewWebViewRequested", [value = std::move(value)](const winrt::Microsoft::ReactNative::IJSValueWriter writer) {
      winrt::Microsoft::ReactNative::WriteValue(writer, value);
    });
  }

  void onJavaScriptDialogRequested(OnJavaScriptDialogRequested &&value) const {
    m_eventEmitter.DispatchEvent(L"javaScriptDialogRequested", [value = std::move(value)](const winrt::Microsoft::ReactNative::IJSValueWriter writer) {
      winrt::Microsoft::ReactNative::WriteValue(writer, value);
    });
  }

  void onJavaScriptDialogDismissed(OnJavaScriptDialogDismissed &&value) const {
    m_eventEmitter.DispatchEvent(L"javaScriptDialogDismissed", [value = std::move(value)](const winrt::Microsoft::ReactNative::IJSValueWriter writer) {
      winrt::Microsoft::ReactNative::WriteValue(writer, value);
    });
  }

  void onContextMenuRequested(OnContextMenuRequested &&value) const {
    m_eventEmitter.DispatchEvent(L"contextMenuRequested", [value = std::move(value)](const winrt::Microsoft::ReactNative::IJSValueWriter writer) {
      winrt::Microsoft::ReactNative::WriteValue(writer, value);
    });
  }

  void onContextMenuItemSelected(OnContextMenuItemSelected &&value) const {
    m_eventEmitter.DispatchEvent(L"contextMenuItemSelected", [value = std::move(value)](const winrt::Microsoft::ReactNative::IJSValueWriter writer) {
      winrt::Microsoft::ReactNative::WriteValue(writer, value);
    });
  }

 private:
  winrt::Microsoft::ReactNative::EventEmitter m_eventEmitter{nullptr};
};

template<typename TUserData>
struct BaseServoView {

  virtual void UpdateProps(
    const winrt::Microsoft::ReactNative::ComponentView &/*view*/,
    const winrt::com_ptr<ServoViewProps> &newProps,
    const winrt::com_ptr<ServoViewProps> &/*oldProps*/) noexcept {
    m_props = newProps;
  }

  // UpdateLayoutMetrics will only be called if this method is overridden
  virtual void UpdateLayoutMetrics(
    const winrt::Microsoft::ReactNative::ComponentView &/*view*/,
    const winrt::Microsoft::ReactNative::LayoutMetrics &/*newLayoutMetrics*/,
    const winrt::Microsoft::ReactNative::LayoutMetrics &/*oldLayoutMetrics*/) noexcept {
  }

  // UpdateState will only be called if this method is overridden
  virtual void UpdateState(
    const winrt::Microsoft::ReactNative::ComponentView &/*view*/,
    const winrt::Microsoft::ReactNative::IComponentState &/*newState*/) noexcept {
  }

  virtual void UpdateEventEmitter(const std::shared_ptr<ServoViewEventEmitter> &eventEmitter) noexcept {
    m_eventEmitter = eventEmitter;
  }

  // MountChildComponentView will only be called if this method is overridden
  virtual void MountChildComponentView(const winrt::Microsoft::ReactNative::ComponentView &/*view*/,
           const winrt::Microsoft::ReactNative::MountChildComponentViewArgs &/*args*/) noexcept {
  }

  // UnmountChildComponentView will only be called if this method is overridden
  virtual void UnmountChildComponentView(const winrt::Microsoft::ReactNative::ComponentView &/*view*/,
           const winrt::Microsoft::ReactNative::UnmountChildComponentViewArgs &/*args*/) noexcept {
  }

  // Initialize will only be called if this method is overridden
  virtual void Initialize(const winrt::Microsoft::ReactNative::ComponentView &/*view*/) noexcept {
  }

  // CreateVisual will only be called if this method is overridden
  virtual winrt::Microsoft::UI::Composition::Visual CreateVisual(const winrt::Microsoft::ReactNative::ComponentView &view) noexcept {
    return view.as<winrt::Microsoft::ReactNative::Composition::ComponentView>().Compositor().CreateSpriteVisual();
  }

  // FinalizeUpdate will only be called if this method is overridden
  virtual void FinalizeUpdate(const winrt::Microsoft::ReactNative::ComponentView &/*view*/,
                                        winrt::Microsoft::ReactNative::ComponentViewUpdateMask /*mask*/) noexcept {
  }

  // CreateAutomationPeer will only be called if this method is overridden
  virtual winrt::Windows::Foundation::IInspectable CreateAutomationPeer(const winrt::Microsoft::ReactNative::ComponentView & /*view*/,
                                        const winrt::Microsoft::ReactNative::CreateAutomationPeerArgs& /*args*/) noexcept {
    return nullptr;
  }

  // You must provide an implementation of this method to handle the "sendControllerCommand" command
  virtual void HandleSendControllerCommandCommand(std::string commandJson) noexcept = 0;

  void HandleCommand(const winrt::Microsoft::ReactNative::ComponentView &view, const winrt::Microsoft::ReactNative::HandleCommandArgs& args) noexcept {
    auto userData = view.UserData().as<TUserData>();
    auto commandName = args.CommandName();
    if (commandName == L"sendControllerCommand") {
      std::string commandJson;
      winrt::Microsoft::ReactNative::ReadArgs(args.CommandArgs(), commandJson);
      userData->HandleSendControllerCommandCommand(commandJson);
      return;
    }
  }

  const std::shared_ptr<ServoViewEventEmitter>& EventEmitter() const { return m_eventEmitter; }
  const winrt::com_ptr<ServoViewProps>& Props() const { return m_props; }

private:
  winrt::com_ptr<ServoViewProps> m_props;
  std::shared_ptr<ServoViewEventEmitter> m_eventEmitter;
};

template <typename TUserData>
void RegisterServoViewNativeComponent(
    winrt::Microsoft::ReactNative::IReactPackageBuilder const &packageBuilder,
    std::function<void(const winrt::Microsoft::ReactNative::Composition::IReactCompositionViewComponentBuilder&)> builderCallback) noexcept {
  packageBuilder.as<winrt::Microsoft::ReactNative::IReactPackageBuilderFabric>().AddViewComponent(
      L"ServoView", [builderCallback](winrt::Microsoft::ReactNative::IReactViewComponentBuilder const &builder) noexcept {
        auto compBuilder = builder.as<winrt::Microsoft::ReactNative::Composition::IReactCompositionViewComponentBuilder>();

        builder.SetCreateProps([](winrt::Microsoft::ReactNative::ViewProps props,
                              const winrt::Microsoft::ReactNative::IComponentProps& cloneFrom) noexcept {
            return winrt::make<ServoViewProps>(props, cloneFrom); 
        });

        builder.SetUpdatePropsHandler([](const winrt::Microsoft::ReactNative::ComponentView &view,
                                     const winrt::Microsoft::ReactNative::IComponentProps &newProps,
                                     const winrt::Microsoft::ReactNative::IComponentProps &oldProps) noexcept {
            auto userData = view.UserData().as<TUserData>();
            userData->UpdateProps(view, newProps ? newProps.as<ServoViewProps>() : nullptr, oldProps ? oldProps.as<ServoViewProps>() : nullptr);
        });

        compBuilder.SetUpdateLayoutMetricsHandler([](const winrt::Microsoft::ReactNative::ComponentView &view,
                                      const winrt::Microsoft::ReactNative::LayoutMetrics &newLayoutMetrics,
                                      const winrt::Microsoft::ReactNative::LayoutMetrics &oldLayoutMetrics) noexcept {
            auto userData = view.UserData().as<TUserData>();
            userData->UpdateLayoutMetrics(view, newLayoutMetrics, oldLayoutMetrics);
        });

        builder.SetUpdateEventEmitterHandler([](const winrt::Microsoft::ReactNative::ComponentView &view,
                                     const winrt::Microsoft::ReactNative::EventEmitter &eventEmitter) noexcept {
          auto userData = view.UserData().as<TUserData>();
          userData->UpdateEventEmitter(std::make_shared<ServoViewEventEmitter>(eventEmitter));
        });

        if CONSTEXPR_SUPPORTED_ON_VIRTUAL_FN_ADDRESS (&TUserData::FinalizeUpdate != &BaseServoView<TUserData>::FinalizeUpdate) {
            builder.SetFinalizeUpdateHandler([](const winrt::Microsoft::ReactNative::ComponentView &view,
                                     winrt::Microsoft::ReactNative::ComponentViewUpdateMask mask) noexcept {
            auto userData = view.UserData().as<TUserData>();
            userData->FinalizeUpdate(view, mask);
          });
        } 

        if CONSTEXPR_SUPPORTED_ON_VIRTUAL_FN_ADDRESS (&TUserData::UpdateState != &BaseServoView<TUserData>::UpdateState) {
          builder.SetUpdateStateHandler([](const winrt::Microsoft::ReactNative::ComponentView &view,
                                     const winrt::Microsoft::ReactNative::IComponentState &newState) noexcept {
            auto userData = view.UserData().as<TUserData>();
            userData->UpdateState(view, newState);
          });
        }

        builder.SetCustomCommandHandler([](const winrt::Microsoft::ReactNative::ComponentView &view,
                                          const winrt::Microsoft::ReactNative::HandleCommandArgs& args) noexcept {
          auto userData = view.UserData().as<TUserData>();
          userData->HandleCommand(view, args);
        });

        if CONSTEXPR_SUPPORTED_ON_VIRTUAL_FN_ADDRESS (&TUserData::MountChildComponentView != &BaseServoView<TUserData>::MountChildComponentView) {
          builder.SetMountChildComponentViewHandler([](const winrt::Microsoft::ReactNative::ComponentView &view,
                                      const winrt::Microsoft::ReactNative::MountChildComponentViewArgs &args) noexcept {
            auto userData = view.UserData().as<TUserData>();
            return userData->MountChildComponentView(view, args);
          });
        }

        if CONSTEXPR_SUPPORTED_ON_VIRTUAL_FN_ADDRESS (&TUserData::UnmountChildComponentView != &BaseServoView<TUserData>::UnmountChildComponentView) {
          builder.SetUnmountChildComponentViewHandler([](const winrt::Microsoft::ReactNative::ComponentView &view,
                                      const winrt::Microsoft::ReactNative::UnmountChildComponentViewArgs &args) noexcept {
            auto userData = view.UserData().as<TUserData>();
            return userData->UnmountChildComponentView(view, args);
          });
        }

        if CONSTEXPR_SUPPORTED_ON_VIRTUAL_FN_ADDRESS (&TUserData::CreateAutomationPeer != &BaseServoView<TUserData>::CreateAutomationPeer) {
            builder.SetCreateAutomationPeerHandler([](const winrt::Microsoft::ReactNative::ComponentView &view,
                                     const winrt::Microsoft::ReactNative::CreateAutomationPeerArgs& args) noexcept {
            auto userData = view.UserData().as<TUserData>();
            return userData->CreateAutomationPeer(view, args);
          });
        } 

        compBuilder.SetViewComponentViewInitializer([](const winrt::Microsoft::ReactNative::ComponentView &view) noexcept {
          auto userData = winrt::make_self<TUserData>();
          if CONSTEXPR_SUPPORTED_ON_VIRTUAL_FN_ADDRESS (&TUserData::Initialize != &BaseServoView<TUserData>::Initialize) {
            userData->Initialize(view);
          }
          view.UserData(*userData);
        });

        if CONSTEXPR_SUPPORTED_ON_VIRTUAL_FN_ADDRESS (&TUserData::CreateVisual != &BaseServoView<TUserData>::CreateVisual) {
          compBuilder.SetCreateVisualHandler([](const winrt::Microsoft::ReactNative::ComponentView &view) noexcept {
            auto userData = view.UserData().as<TUserData>();
            return userData->CreateVisual(view);
          });
        }

        // Allow app to further customize the builder
        if (builderCallback) {
          builderCallback(compBuilder);
        }
      });
}

} // namespace ServoKitCodegen

#endif // #ifdef RNW_NEW_ARCH
