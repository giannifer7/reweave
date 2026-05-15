use super::*;

impl<'a> Lexer<'a> {
    // ── Low-level cursor ──────────────────────────────────────────────────

    pub(in crate::lexer) fn peek_byte(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    pub(in crate::lexer) fn advance(&mut self) -> Option<u8> {
        let b = self.bytes.get(self.pos).copied()?;
        self.pos += 1;
        Some(b)
    }

    pub(in crate::lexer) fn starts_with_sigil(&self) -> bool {
        self.bytes[self.pos..].starts_with(&self.sigil_bytes)
    }

    pub(in crate::lexer) fn advance_sigil(&mut self) -> bool {
        if self.starts_with_sigil() {
            self.pos += self.sigil_bytes.len();
            true
        } else {
            false
        }
    }

    /// Advance past the rest of the current line (through `\n` or to EOF).
    pub(in crate::lexer) fn skip_line_comment(&mut self) {
        let rest = &self.bytes[self.pos..];
        match memchr(b'\n', rest) {
            Some(i) => self.pos += i + 1,
            None => self.pos = self.bytes.len(),
        }
    }

    /// Returns the byte index just past the end of an identifier starting at `start`.
    pub(in crate::lexer) fn get_identifier_end(&self, start: usize) -> usize {
        let bytes = self.bytes;
        if start >= bytes.len() || !is_identifier_start(bytes[start]) {
            return start;
        }
        let mut end = start + 1;
        while end < bytes.len() && is_identifier_continue(bytes[end]) {
            end += 1;
        }
        end
    }

    pub(in crate::lexer) fn starts_with_bytes(&self, pat: &[u8]) -> bool {
        self.bytes[self.pos..].starts_with(pat)
    }

    /// Extract the identifier tag from a `%tag{` or `%tag}` position.
    /// `pct_start` is the byte offset of `%`. Returns `""` for anonymous `%{`/`%}`.
    pub(in crate::lexer) fn block_tag_at(&self, pct_start: usize) -> &str {
        let start = pct_start + self.sigil_bytes.len(); // skip sigil bytes
        let mut end = start;
        while end < self.bytes.len() && is_identifier_continue(self.bytes[end]) {
            end += 1;
        }
        std::str::from_utf8(&self.bytes[start..end]).unwrap_or("")
    }
}
