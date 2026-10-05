import { spawnSync } from 'node:child_process';
import {
  existsSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

import { expect, test } from 'bun:test';

const darwinTest = process.platform === 'darwin' ? test : test.skip;

function expectIOSPodProjectionUsesWKWebViewAndPortableController(
  podspec: string
) {
  expect(
    podspec
      .match(/^\s*s\.(?:source_files|frameworks|vendored_frameworks)\s*=.*$/gm)
      ?.map((line) => line.trim())
  ).toEqual([
    's.source_files = "ios/ExplorerView.mm"',
    's.frameworks   = "WebKit"',
    's.vendored_frameworks = "ios/ServoKitController.xcframework"',
  ]);
  expect(podspec).toContain(
    's.platforms    = { :ios => min_ios_version_supported }'
  );
  expect(
    [
      ...podspec.matchAll(
        /^\s*s\.dependency\s*(?:\(\s*)?["']([^"']+)["']/gm
      ),
    ].map(([, dependency]) => dependency)
  ).toEqual([
    'React-Core',
    'React-Codegen',
    'RCT-Folly',
    'RCTRequired',
    'RCTTypeSafety',
    'ReactCommon/turbomodule/core',
  ]);
  expect(podspec).not.toMatch(/:\s*(?:osx|macos)\b/);
  expect(podspec).not.toMatch(/(?:macOS|EXPLORERKIT_BUILD_FROM_SOURCE|ExplorerKitMacOSBinary)/);
  expect(podspec).not.toMatch(/\b(?:vendored_libraries|prepare_command|script_phase)\b/);
  expect(podspec).not.toMatch(/\b(?:cargo|curl|download|postinstall)\b/i);
  expect(podspec).not.toMatch(/\bsystem\s*\(/);
}

test('iOS Fabric commands use the generated ExplorerView handler', () => {
  const source = readFileSync(
    new URL('../../ios/ExplorerView.mm', import.meta.url),
    'utf8'
  );

  expect(source).toContain(`- (void)handleCommand:(const NSString *)commandName args:(const NSArray *)args
{
  RCTExplorerViewHandleCommand(self, commandName, args);
}`);
  expect(source).toContain(`ExplorerViewEventEmitter::OnLoadStatusChanged event = {
      .status = ExplorerKitStdString(status),
  };`);
  expect(source).toContain('NSNumber *timeoutMs = [effect[@"timeoutMs"]');
  expect(source).toContain('if ([_rawURLProp isEqualToString:nextURL])');
  expect(source).toContain('nextGeneration != _controllerGeneration + 1');
  expect(source).toContain('_webView.navigationDelegate = nil;');
  expect(source.indexOf('_webView.navigationDelegate = nil;')).toBeLessThan(
    source.indexOf('[_webView stopLoading];')
  );
  expect(source).toContain(`for (NSString *dialogId in dialogHandlers) {
    ExplorerKitJavaScriptDialogResolutionHandler handler = dialogHandlers[dialogId];
    handler(NO, nil);
    if (self.onJavaScriptDialogDismissed != nil) {
      self.onJavaScriptDialogDismissed(dialogId);
    }
  }`);
  expect(source).toContain(`if (object != _webView) {
    return;
  }`);
  expect(source).toContain(`if (webView != _webView) {
    completionHandler();
    return;
  }`);
  expect(source).toContain(`if (webView != _webView) {
    completionHandler(NO);
    return;
  }`);
  expect(source).toContain(`if (webView != _webView) {
    completionHandler(nil);
    return;
  }`);
  expect(source).toContain(`if (webView != _webView) {
    decisionHandler(WKNavigationActionPolicyAllow);
    return;
  }`);
  expect(source).toContain(`- (void)prepareForReuse
{
  if (![self dispatchControllerEnvelope:@{@"version" : @1, @"lifecycle" : @"reset"}]) {
    [self replaceWebView];
  }
}`);
  expect(source).toContain(
    '_pendingJavaScriptEvaluationIdsByRequestId[requestId] = evaluationId;'
  );
  expect(source).toContain('[self retireControllerAfterFailure:evaluationIdOnError];');
  expect(source).toContain(`  [self createController];
  [self failNativeCompletionsWithEvaluationId:evaluationId];`);
  expect(source).toContain(
    '(_controllerHandle == 0 && ![self createController])'
  );
  expect(source).not.toContain('_pendingJavaScriptEvaluationRequestIds');
  expect(source).not.toContain('ServoKitControllerFallbackSeconds');
});

test('iOS CocoaPods projection selects WKWebView and the Servo-free controller', () => {
  const podspec = readFileSync(
    new URL('../../ExplorerKit.podspec', import.meta.url),
    'utf8'
  );
  const packageJson = JSON.parse(
    readFileSync(new URL('../../package.json', import.meta.url), 'utf8')
  ) as { scripts: Record<string, string> };

  expectIOSPodProjectionUsesWKWebViewAndPortableController(podspec);
  expect(podspec).toContain('s.dependency "React-Core"');
  expect(packageJson.scripts).not.toHaveProperty('preinstall');
  expect(packageJson.scripts).not.toHaveProperty('install');
  expect(packageJson.scripts).not.toHaveProperty('postinstall');
  expect(Object.values(packageJson.scripts).join('\n')).not.toContain(
    'prepare-macos-source'
  );
});

darwinTest('an isolated iOS CocoaPods resolution does not require the macOS binary pod', () => {
  const packageRoot = dirname(
    dirname(dirname(fileURLToPath(import.meta.url)))
  );
  const packageJson = JSON.parse(
    readFileSync(new URL('../../package.json', import.meta.url), 'utf8')
  ) as { version: string };
  const temporaryRoot = mkdtempSync(
    join(tmpdir(), 'explorerkit-ios-pod-resolution-')
  );
  const specRepository = join(temporaryRoot, 'specs.git');
  const podfile = `source ${JSON.stringify(pathToFileURL(specRepository).href)}

install! 'cocoapods', :integrate_targets => false
platform :ios, '15.1'

class << ::Pod
  def min_ios_version_supported
    '15.1'
  end

  def install_modules_dependencies(_spec)
  end
end

target 'ExplorerKitIOSProjection' do
  pod 'ExplorerKit', :path => ${JSON.stringify(packageRoot)}
end
`;

  try {
    expect(
      spawnSync('git', ['init', '--bare', specRepository], {
        stdio: 'ignore',
      }).status
    ).toBe(0);
    writeFileSync(join(temporaryRoot, 'Podfile'), podfile);
    const result = spawnSync('pod', ['install', '--no-repo-update'], {
      cwd: temporaryRoot,
      env: {
        ...process.env,
        COCOAPODS_DISABLE_STATS: 'true',
        CP_HOME_DIR: join(temporaryRoot, 'cocoapods-home'),
      },
      stdio: 'inherit',
    });

    expect(result.status).toBe(0);
    expect(existsSync(join(temporaryRoot, 'Podfile.lock'))).toBe(true);
    const lockfile = readFileSync(join(temporaryRoot, 'Podfile.lock'), 'utf8');
    expect(lockfile).toContain(`- ExplorerKit (${packageJson.version})`);
    expect(lockfile).not.toContain('ExplorerKitMacOS');
    expect(lockfile).not.toContain('ExplorerKitMacOSBinary');
  } finally {
    rmSync(temporaryRoot, { force: true, recursive: true });
  }
});
