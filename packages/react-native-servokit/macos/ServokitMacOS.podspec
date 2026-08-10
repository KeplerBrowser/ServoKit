require "json"

package = JSON.parse(File.read(File.join(__dir__, "..", "package.json")))
folly_compiler_flags = '-DFOLLY_NO_CONFIG -DFOLLY_MOBILE=1 -DFOLLY_USE_LIBCPP=1 -Wno-comma -Wno-shorten-64-to-32'

Pod::Spec.new do |s|
  s.name         = "ServokitMacOS"
  s.version      = package["version"]
  s.summary      = "Experimental React Native macOS ServoKit adapter"
  s.homepage     = package["homepage"]
  s.license      = package["license"]
  s.authors      = package["author"]

  s.platform     = :osx, "14.0"
  s.source       = { :git => "https://github.com/KeplerBrowser/ServoKit.git", :tag => "#{s.version}" }
  s.source_files = "ServoView.mm"
  s.frameworks   = "AppKit"

  if ENV["SERVOKIT_BUILD_FROM_SOURCE"] == "1"
    s.vendored_frameworks = "../.servokit-source/current/ServoKit.xcframework"
  else
    s.dependency "ServoKitMacOSBinary", package["version"]
  end

  if respond_to?(:install_modules_dependencies, true)
    install_modules_dependencies(s)
  else
    s.dependency "React-Core"

    if ENV['RCT_NEW_ARCH_ENABLED'] == '1' then
      s.compiler_flags = folly_compiler_flags + " -DRCT_NEW_ARCH_ENABLED=1"
      s.pod_target_xcconfig = {
        "HEADER_SEARCH_PATHS" => "\"$(PODS_ROOT)/boost\"",
        "OTHER_CPLUSPLUSFLAGS" => "-DFOLLY_NO_CONFIG -DFOLLY_MOBILE=1 -DFOLLY_USE_LIBCPP=1",
        "CLANG_CXX_LANGUAGE_STANDARD" => "c++17"
      }
      s.dependency "React-Codegen"
      s.dependency "RCT-Folly"
      s.dependency "RCTRequired"
      s.dependency "RCTTypeSafety"
      s.dependency "ReactCommon/turbomodule/core"
    end
  end
end
