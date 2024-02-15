use assert_cmd::Command;

/// Construct a Command from a given string.
pub(crate) fn cmd<S: AsRef<str>>(cmd: S) -> Command {
    let args: Vec<_> = cmd.as_ref().split_whitespace().collect();
    let mut cmd = Command::cargo_bin(args[0]).unwrap();
    cmd.args(&args[1..]);
    // disable config loading by default
    cmd.env("BITE_NO_CONFIG", "1");
    cmd
}
