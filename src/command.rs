use itertools::Itertools;
use shell_escape::escape;

use crate::syntax::{Highlighter, Language, Theme};

/// Converts a command and its arguments into a title string.
pub fn to_title(
    command: Option<impl AsRef<str>>,
    args: impl IntoIterator<Item = impl AsRef<str>>,
) -> Option<String> {
    Some(
        std::iter::once(escape(command?.as_ref().into()))
            .chain(
                args.into_iter()
                    .map(|arg| escape(arg.as_ref().to_owned().into())),
            )
            .join(" "),
    )
}

/// Formats a command line with syntax highlighting for display in the terminal surface.
///
/// Uses tree-sitter-based syntax highlighting to colorize the command as bash.
/// The prompt is rendered as-is, followed by the highlighted command and a trailing newline.
pub fn to_terminal(
    prompt: impl AsRef<str>,
    command: impl AsRef<str>,
    args: impl IntoIterator<Item = impl AsRef<str>>,
    theme: Option<Theme>,
) -> Vec<u8> {
    let prompt = prompt.as_ref();
    let command = command_string(command, args);

    let highlighter = Highlighter::new(Language::Bash, theme);

    let mut output = Vec::new();
    output.extend(b"\x1b[35m");
    output.extend(prompt.as_bytes());
    output.extend(b"\x1b[0m");
    highlighter.format(&command, &mut output).unwrap();
    output.push(b'\n');

    output
}

fn command_string(
    command: impl AsRef<str>,
    args: impl IntoIterator<Item = impl AsRef<str>>,
) -> String {
    std::iter::once(escape(command.as_ref().into()))
        .chain(
            args.into_iter()
                .map(|arg| escape(arg.as_ref().to_owned().into())),
        )
        .join(" ")
}

#[cfg(test)]
mod tests;
