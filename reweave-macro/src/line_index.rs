use memchr::memchr_iter;
/// Converts byte offsets to 1-indexed (line, column) pairs on demand.
///
/// Construct once per source string; each lookup is O(log n) via binary search
/// over the cached newline positions.
pub struct LineIndex {
    /// Sorted byte offsets of every `\n` in the source.
    newlines: Vec<usize>,
}

impl LineIndex {
    pub fn new(source: &str) -> Self {
        let newlines = memchr_iter(b'\n', source.as_bytes()).collect();
        Self { newlines }
    }

    pub fn from_bytes(source: &[u8]) -> Self {
        let newlines = memchr_iter(b'\n', source).collect();
        Self { newlines }
    }

    /// Returns the 1-indexed `(line, column)` for a byte offset.
    /// Column is a 1-indexed byte offset within the line.
    pub fn line_col(&self, pos: usize) -> (usize, usize) {
        // Number of newlines strictly before `pos` = the line index (0-based).
        let i = self.newlines.partition_point(|&nl| nl < pos);
        let line_start = if i == 0 { 0 } else { self.newlines[i - 1] + 1 };
        (i + 1, pos - line_start + 1)
    }
}
#[cfg(test)]
mod tests;
