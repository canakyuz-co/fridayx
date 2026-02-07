use std::collections::HashMap;

use regex::RegexBuilder;
use ropey::Rope;

#[derive(Clone, Copy)]
pub(crate) struct EditorSearchOptions {
    pub(crate) match_case: bool,
    pub(crate) whole_word: bool,
    pub(crate) is_regex: bool,
}

#[derive(Clone)]
pub(crate) struct EditorBufferSnapshot {
    pub(crate) buffer_id: u64,
    pub(crate) path: String,
    pub(crate) version: u64,
    pub(crate) line_count: u32,
    pub(crate) byte_len: u64,
    pub(crate) is_dirty: bool,
}

#[derive(Clone)]
pub(crate) struct EditorRangeRead {
    pub(crate) version: u64,
    pub(crate) text: String,
}

#[derive(Clone)]
pub(crate) struct EditorSearchMatch {
    pub(crate) line: u32,
    pub(crate) column: u32,
    pub(crate) line_text: String,
    pub(crate) match_text: Option<String>,
}

struct EditorBuffer {
    workspace_id: String,
    path: String,
    rope: Rope,
    version: u64,
    is_dirty: bool,
}

pub(crate) struct EditorCore {
    next_buffer_id: u64,
    buffers: HashMap<u64, EditorBuffer>,
}

impl Default for EditorCore {
    fn default() -> Self {
        Self {
            next_buffer_id: 1,
            buffers: HashMap::new(),
        }
    }
}

impl EditorCore {
    pub(crate) fn open_buffer(
        &mut self,
        workspace_id: String,
        path: String,
        content: String,
    ) -> EditorBufferSnapshot {
        let buffer_id = self.next_buffer_id;
        self.next_buffer_id = self.next_buffer_id.saturating_add(1);
        let rope = Rope::from_str(&content);
        let snapshot = build_snapshot(buffer_id, &path, 1, &rope, false);
        self.buffers.insert(
            buffer_id,
            EditorBuffer {
                workspace_id,
                path,
                rope,
                version: 1,
                is_dirty: false,
            },
        );
        snapshot
    }

    pub(crate) fn close_buffer(&mut self, buffer_id: u64) -> Result<(), String> {
        self.buffers
            .remove(&buffer_id)
            .map(|_| ())
            .ok_or_else(|| "buffer not found".to_string())
    }

    pub(crate) fn snapshot(&self, buffer_id: u64) -> Result<EditorBufferSnapshot, String> {
        let buffer = self
            .buffers
            .get(&buffer_id)
            .ok_or_else(|| "buffer not found".to_string())?;
        Ok(build_snapshot(
            buffer_id,
            &buffer.path,
            buffer.version,
            &buffer.rope,
            buffer.is_dirty,
        ))
    }

    pub(crate) fn read_range(
        &self,
        buffer_id: u64,
        start_line: u32,
        end_line: u32,
    ) -> Result<EditorRangeRead, String> {
        let buffer = self
            .buffers
            .get(&buffer_id)
            .ok_or_else(|| "buffer not found".to_string())?;
        let len_lines = buffer.rope.len_lines() as u32;
        if len_lines == 0 {
            return Ok(EditorRangeRead {
                version: buffer.version,
                text: String::new(),
            });
        }
        let clamped_start = start_line.clamp(1, len_lines);
        let clamped_end = end_line.clamp(clamped_start, len_lines);
        let mut text = String::new();
        for line_idx in (clamped_start - 1)..clamped_end {
            text.push_str(&buffer.rope.line(line_idx as usize).to_string());
        }
        Ok(EditorRangeRead {
            version: buffer.version,
            text,
        })
    }

    pub(crate) fn apply_delta(
        &mut self,
        buffer_id: u64,
        expected_version: u64,
        start_offset: u64,
        end_offset: u64,
        text: &str,
    ) -> Result<u64, String> {
        let buffer = self
            .buffers
            .get_mut(&buffer_id)
            .ok_or_else(|| "buffer not found".to_string())?;
        if buffer.version != expected_version {
            return Err("version mismatch".to_string());
        }
        let rope_len = buffer.rope.len_bytes() as u64;
        if start_offset > end_offset || end_offset > rope_len {
            return Err("invalid delta range".to_string());
        }
        let start_char = buffer.rope.byte_to_char(start_offset as usize);
        let end_char = buffer.rope.byte_to_char(end_offset as usize);
        buffer.rope.remove(start_char..end_char);
        buffer.rope.insert(start_char, text);
        buffer.version = buffer.version.saturating_add(1);
        buffer.is_dirty = true;
        Ok(buffer.version)
    }

    pub(crate) fn mark_saved(&mut self, buffer_id: u64) -> Result<(), String> {
        let buffer = self
            .buffers
            .get_mut(&buffer_id)
            .ok_or_else(|| "buffer not found".to_string())?;
        buffer.is_dirty = false;
        Ok(())
    }

    pub(crate) fn replace_content(
        &mut self,
        buffer_id: u64,
        content: String,
    ) -> Result<EditorBufferSnapshot, String> {
        let buffer = self
            .buffers
            .get_mut(&buffer_id)
            .ok_or_else(|| "buffer not found".to_string())?;
        buffer.rope = Rope::from_str(&content);
        buffer.version = buffer.version.saturating_add(1);
        buffer.is_dirty = false;
        Ok(build_snapshot(
            buffer_id,
            &buffer.path,
            buffer.version,
            &buffer.rope,
            buffer.is_dirty,
        ))
    }

    pub(crate) fn export_content(&self, buffer_id: u64) -> Result<String, String> {
        let buffer = self
            .buffers
            .get(&buffer_id)
            .ok_or_else(|| "buffer not found".to_string())?;
        Ok(buffer.rope.to_string())
    }

    pub(crate) fn search_in_buffer(
        &self,
        buffer_id: u64,
        query: &str,
        options: EditorSearchOptions,
        max_results: usize,
    ) -> Result<Vec<EditorSearchMatch>, String> {
        let buffer = self
            .buffers
            .get(&buffer_id)
            .ok_or_else(|| "buffer not found".to_string())?;
        let trimmed_query = query.trim();
        if trimmed_query.is_empty() || max_results == 0 {
            return Ok(Vec::new());
        }
        let regex = if options.is_regex {
            let pattern = if options.whole_word {
                format!(r"\b(?:{trimmed_query})\b")
            } else {
                trimmed_query.to_string()
            };
            Some(
                RegexBuilder::new(&pattern)
                    .case_insensitive(!options.match_case)
                    .build()
                    .map_err(|err| format!("invalid regex query: {err}"))?,
            )
        } else {
            None
        };

        let query_cmp = if options.match_case {
            trimmed_query.to_string()
        } else {
            trimmed_query.to_lowercase()
        };
        let mut results = Vec::new();
        for line_idx in 0..buffer.rope.len_lines() {
            let line_text = buffer
                .rope
                .line(line_idx)
                .to_string()
                .trim_end_matches(['\n', '\r'])
                .to_string();
            collect_line_matches(
                &line_text,
                &query_cmp,
                options,
                regex.as_ref(),
                line_idx as u32 + 1,
                max_results,
                &mut results,
            );
            if results.len() >= max_results {
                break;
            }
        }
        Ok(results)
    }

    pub(crate) fn buffer_path(&self, buffer_id: u64) -> Result<(String, String), String> {
        let buffer = self
            .buffers
            .get(&buffer_id)
            .ok_or_else(|| "buffer not found".to_string())?;
        Ok((buffer.workspace_id.clone(), buffer.path.clone()))
    }
}

fn build_snapshot(
    buffer_id: u64,
    path: &str,
    version: u64,
    rope: &Rope,
    is_dirty: bool,
) -> EditorBufferSnapshot {
    EditorBufferSnapshot {
        buffer_id,
        path: path.to_string(),
        version,
        line_count: rope.len_lines() as u32,
        byte_len: rope.len_bytes() as u64,
        is_dirty,
    }
}

fn collect_line_matches(
    line_text: &str,
    query: &str,
    options: EditorSearchOptions,
    regex: Option<&regex::Regex>,
    line: u32,
    max_results: usize,
    results: &mut Vec<EditorSearchMatch>,
) {
    if results.len() >= max_results {
        return;
    }
    if let Some(regex) = regex {
        for m in regex.find_iter(line_text) {
            if results.len() >= max_results {
                break;
            }
            let column = line_text[..m.start()].chars().count() as u32 + 1;
            results.push(EditorSearchMatch {
                line,
                column,
                line_text: line_text.to_string(),
                match_text: Some(m.as_str().to_string()),
            });
        }
        return;
    }

    let target = if options.match_case {
        line_text.to_string()
    } else {
        line_text.to_lowercase()
    };
    let mut start = 0usize;
    while let Some(found) = target[start..].find(query) {
        if results.len() >= max_results {
            break;
        }
        let start_byte = start + found;
        let end_byte = start_byte + query.len();
        if options.whole_word && !is_whole_word_match(line_text, start_byte, end_byte) {
            start = end_byte;
            continue;
        }
        let column = line_text[..start_byte].chars().count() as u32 + 1;
        results.push(EditorSearchMatch {
            line,
            column,
            line_text: line_text.to_string(),
            match_text: Some(line_text[start_byte..end_byte].to_string()),
        });
        start = end_byte;
    }
}

fn is_whole_word_match(text: &str, start_byte: usize, end_byte: usize) -> bool {
    let before = text[..start_byte].chars().next_back();
    let after = text[end_byte..].chars().next();
    !is_word_char(before) && !is_word_char(after)
}

fn is_word_char(ch: Option<char>) -> bool {
    ch.is_some_and(|value| value.is_alphanumeric() || value == '_')
}

