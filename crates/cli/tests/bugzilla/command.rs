/// Construct a Command from a given string.
macro_rules! cmd {
    ($cmd:expr) => {{
        let cmd = format!($cmd);
        let mut args = shlex::split(&cmd).unwrap_or_default().into_iter();
        let cmd = args.next().unwrap();
        let mut cmd = assert_cmd::Command::cargo_bin(cmd).unwrap();
        cmd.args(args);
        // disable config loading by default
        cmd.env("BUGBITE_CONFIG", "false");
        cmd
    }};
}
pub(crate) use cmd;
