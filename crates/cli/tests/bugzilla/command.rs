/// Construct a bite command using a given argument string.
macro_rules! cmd {
    ($args:expr) => {{
        let args = format!($args);
        let mut args = shlex::split(&args).unwrap_or_default().into_iter();
        let mut cmd = match args.next().as_deref() {
            Some("bite") => assert_cmd::cargo::cargo_bin_cmd!("bite"),
            Some(x) => panic!("unknown command: {x}"),
            None => panic!("invalid command"),
        };
        cmd.args(args);
        // disable config loading by default
        cmd.env("BUGBITE_CONFIG_DIR", "false");
        cmd
    }};
}
pub(crate) use cmd;
