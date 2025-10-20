// std imports
use std::{fmt, io};

// third-party imports
use owo_colors::OwoColorize;

/// A helper for formatting lists of assets.
pub struct Formatter<O> {
    width: Option<usize>,
    output: O,
}

impl<O> Formatter<O>
where
    O: io::Write,
{
    /// Creates a new `Formatter` with the given output.
    ///
    /// If the output is a terminal, it attempts to determine the terminal width.
    ///
    /// # Arguments
    ///
    /// * `output` - The output to write to.
    pub fn new(output: O) -> Self
    where
        O: io::IsTerminal,
    {
        let width = if output.is_terminal() {
            term_size::dimensions().map(|d| d.0)
        } else {
            None
        };

        Self { output, width }
    }

    /// Creates a new `Formatter` with the given output and width.
    ///
    /// # Arguments
    ///
    /// * `output` - The output to write to.
    /// * `width` - The optional width for formatting.
    #[allow(dead_code)]
    pub fn with_width(output: O, width: Option<usize>) -> Self {
        Self { output, width }
    }

    /// Formats a grouped list of items and writes to the output.
    ///
    /// If the width is set, it formats the items into columns. Otherwise, it writes the items in a raw list format.
    ///
    /// # Arguments
    ///
    /// * `groups` - An iterator over groups, where each group is a tuple of a displayable group name and an iterator over items.
    ///
    /// # Errors
    ///
    /// Returns an `io::Result` with an error if writing to the output fails.
    pub fn format_grouped_list<G, V, GI, I>(&mut self, groups: GI) -> io::Result<()>
    where
        GI: IntoIterator<Item = (G, I)>,
        I: IntoIterator<Item = V>,
        G: fmt::Display,
        V: AsRef<str>,
    {
        let Some(width) = self.width else {
            return self
                .format_raw_list(groups.into_iter().map(|x| x.1).flat_map(|x| x.into_iter()));
        };

        let out = &mut self.output;

        let mut groups = groups
            .into_iter()
            .map(|(g, items)| (g, items.into_iter().collect::<Vec<_>>()))
            .collect::<Vec<_>>();

        let max_len = groups
            .iter()
            .map(|x| x.1.iter().map(|x| x.as_ref().len()).max().unwrap_or(0))
            .max()
            .unwrap_or(0);

        let columns = width / (max_len + 4);

        for (group, items) in groups.iter_mut() {
            writeln!(out, "{}:", group.bold())?;

            let rows = items.len().div_ceil(columns);

            for row in 0..rows {
                for col in 0..columns {
                    if let Some(val) = items.get(row + col * rows) {
                        write!(out, "• {:width$}", val.as_ref(), width = max_len + 2)?;
                    }
                }
                writeln!(out)?;
            }
        }
        Ok(())
    }

    /// Formats a raw list of items and writes to the output.
    ///
    /// # Arguments
    ///
    /// * `items` - An iterator over items to be written.
    ///
    /// # Errors
    ///
    /// Returns an `io::Result` with an error if writing to the output fails.
    fn format_raw_list<I, V>(&mut self, items: I) -> io::Result<()>
    where
        I: IntoIterator<Item = V>,
        V: AsRef<str>,
    {
        for item in items {
            writeln!(&mut self.output, "{}", item.as_ref())?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
