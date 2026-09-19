#pragma once

#include "ReactPackageProvider.g.h"

namespace winrt::ServoKit::implementation
{
    struct ReactPackageProvider : ReactPackageProviderT<ReactPackageProvider>
    {
        void CreatePackage(
            winrt::Microsoft::ReactNative::IReactPackageBuilder const &packageBuilder) noexcept;
    };
}

namespace winrt::ServoKit::factory_implementation
{
    struct ReactPackageProvider
        : ReactPackageProviderT<ReactPackageProvider, implementation::ReactPackageProvider>
    {
    };
}
