/// Construct a Command from a given string.
macro_rules! cmd {
    ($cmd:expr) => {{
        let cmd = format!($cmd);
        let args: Vec<_> = cmd.split_whitespace().collect();
        let mut cmd = assert_cmd::Command::cargo_bin(args[0]).unwrap();
        cmd.args(&args[1..]);
        // disable config loading by default
        cmd.env("BITE_NO_CONFIG", "1");
        cmd
    }};
}
pub(crate) use cmd;
