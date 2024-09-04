use assert_cmd::Command;

/// Construct a Command from a given string.
pub(crate) fn cmd<S: AsRef<str>>(cmd: S) -> Command {
    let mut args = shlex::split(cmd.as_ref()).unwrap_or_default().into_iter();
    let cmd = args.next().unwrap();
    let mut cmd = assert_cmd::Command::cargo_bin(cmd).unwrap();
    cmd.args(args);
    // disable config loading by default
    cmd.env("BUGBITE_CONFIG", "false");
    cmd
}
