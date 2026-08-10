type MountedControllerCommands<View> = Readonly<{
  sendControllerCommand(view: View, commandJson: string): void;
}>;

export function sendMountedControllerCommand<View>(
  nativeView: View | null,
  commandJson: string,
  commands: MountedControllerCommands<View>
): boolean {
  if (nativeView == null) {
    return false;
  }

  try {
    commands.sendControllerCommand(nativeView, commandJson);
    return true;
  } catch {
    return false;
  }
}
