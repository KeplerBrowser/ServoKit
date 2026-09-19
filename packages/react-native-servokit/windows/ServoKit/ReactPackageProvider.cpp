#include "pch.h"

#include "ReactPackageProvider.h"

#if __has_include("ReactPackageProvider.g.cpp")
#include "ReactPackageProvider.g.cpp"
#endif

#include "ServoView.h"

using namespace winrt::Microsoft::ReactNative;

namespace winrt::ServoKit::implementation
{
void ReactPackageProvider::CreatePackage(IReactPackageBuilder const &packageBuilder) noexcept
{
    AddAttributedModules(packageBuilder, true);
    RegisterServoViewNativeComponent(packageBuilder);
}
} // namespace winrt::ServoKit::implementation
