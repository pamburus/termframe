use crate::error::{AppInfoProvider, Error};
use std::io::{self, Write};

struct TestAppInfo;
impl AppInfoProvider for TestAppInfo {}

#[test]
fn test_log() {
    let err = Error::Io(std::io::Error::other("test"));
    let mut buf = Vec::new();
    err.log_to(&mut buf, &TestAppInfo).unwrap();
    assert_eq!(
        String::from_utf8(buf).unwrap(),
        "\u{1b}[1m\u{1b}[91merror:\u{1b}[39m\u{1b}[0m test\n"
    );
}

#[test]
fn test_formatter() {
    let mut output = Vec::new();

    struct MockFormatter<W: Write> {
        output: W,
    }

    impl<W: Write> MockFormatter<W> {
        fn new(output: W) -> Self {
            Self { output }
        }

        fn format_grouped_list<I, G, S>(&mut self, groups: I) -> io::Result<()>
        where
            I: IntoIterator<Item = (G, S)>,
            G: AsRef<str>,
            S: IntoIterator,
            S::Item: AsRef<str>,
        {
            for (group, items) in groups {
                writeln!(self.output, "# {}", group.as_ref())?;
                for item in items {
                    writeln!(self.output, "- {}", item.as_ref())?;
                }
            }
            Ok(())
        }
    }

    let mut formatter = MockFormatter::new(&mut output);

    let groups = vec![
        ("Group1", vec!["Item1", "Item2", "Item3"]),
        ("Group2", vec!["ItemA", "ItemB"]),
    ];

    formatter.format_grouped_list(groups).unwrap();

    let output = String::from_utf8(output).unwrap();
    println!("Formatted output: {}", output);

    assert!(output.contains("Group1"));
    assert!(output.contains("Item1"));
    assert!(output.contains("Item2"));
    assert!(output.contains("Item3"));
    assert!(output.contains("Group2"));
    assert!(output.contains("ItemA"));
    assert!(output.contains("ItemB"));
}

#[test]
fn test_error_log() {
    let err = std::io::Error::other("test error");
    let mut buffer = Vec::new();
    let _ = Error::Io(err).log_to(&mut buffer, &TestAppInfo);

    let log_output = String::from_utf8(buffer).unwrap();
    println!("Error log output: {}", log_output);

    assert!(log_output.contains("test error"));
}
