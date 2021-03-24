use clap::{App, ArgMatches, Shell};
use clap::{Arg, SubCommand};
use std::str::FromStr;

pub fn add_common_options(app: App<'static, 'static>) -> App<'static, 'static> {
    app.arg(
        Arg::with_name("v")
            .help("Sets the verbosity level. May be applied multiple times: -vvv")
            .short("v")
            .multiple(true),
    )
    .subcommand(
        SubCommand::with_name("completions")
            .about("Generates completion scripts for your shell")
            .arg(
                Arg::with_name("SHELL")
                    .required(true)
                    .possible_values(&["bash", "fish", "zsh", "powershell"])
                    .help("The shell to generate the script for"),
            ),
    )
}

pub fn get_verbosity(matches: &ArgMatches) -> String {
    let verbosity = matches.occurrences_of("v") as usize;
    if verbosity == 0 {
        // Special case for no verbosity on the command line
        // Try to read from the default environment variable or fall back to error
        // The format of the `RUST_LOG` variable is described in depth here
        // https://docs.rs/env_logger/latest/env_logger/#enabling-logging
        std::env::var("RUST_LOG").unwrap_or_else(|_| "error".to_string())
    } else {
        // A bit of stderrlog compatibility
        // Add log level from the v:s starting with 1 as warning
        match verbosity {
            1 => "warning",
            2 => "info",
            3 => "debug",
            _ => "trace",
        }
        .to_string()
    }
}

pub fn handle_completion(
    command_name: &str,
    matches: &ArgMatches,
    build_cli: &dyn Fn() -> App<'static, 'static>,
) -> bool {
    if let ("completions", Some(sub_matches)) = matches.subcommand() {
        let shell = sub_matches.value_of("SHELL").unwrap();
        build_cli().gen_completions_to(command_name, Shell::from_str(shell).unwrap(), &mut std::io::stdout());
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {

    /// Tries to parse the given string as a `usize` or returns `None` if the input matches the
    /// given `none` string. If the argument is `None` the result will be `None` too.
    pub fn parse_usize_or_none(arg: Option<&str>, none: &str, errormsg: &str) -> Option<usize> {
        if let Some(s) = arg {
            if s.to_lowercase().eq(none) {
                None
            } else {
                Some(s.parse::<usize>().expect(errormsg))
            }
        } else {
            None
        }
    }

    #[test]
    fn usize_or_none_must_handle_none_option() {
        assert_eq!(None, parse_usize_or_none(None, "none", "broken"));
    }

    #[test]
    fn usize_or_none_must_handle_none_string() {
        assert_eq!(None, parse_usize_or_none(Some("nOnE"), "none", "broken"));
    }

    #[test]
    fn usize_or_none_must_handle_usize() {
        assert_eq!(Some(4711), parse_usize_or_none(Some("4711"), "none", "broken"));
    }

    #[test]
    #[should_panic(expected = "broken")]
    fn usize_or_none_must_fail_on_bad_input() {
        assert_eq!(None, parse_usize_or_none(Some("illegal"), "none", "broken"));
    }
}
