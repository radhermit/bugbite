use assert_cmd::Command;

/// Construct a Command from a given string.
pub(crate) fn cmd<S: AsRef<str>>(cmd: S) -> Command {
    let mut args = shlex::split(cmd.as_ref()).unwrap_or_default().into_iter();
    let mut cmd = match args.next().as_deref() {
        Some("bite") => assert_cmd::cargo::cargo_bin_cmd!("bite"),
        Some(x) => panic!("unknown command: {x}"),
        None => panic!("invalid command"),
    };
    cmd.args(args);
    // disable config loading by default
    cmd.env("BUGBITE_CONFIG_DIR", "false");
    cmd
}
