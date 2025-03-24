use std::error::Error;
use std::fmt;

use crate::ast::{Line, Patch};

/// Error that can occur while applying a patch
#[derive(Debug)]
pub enum ApplyError {
    /// The line number in the patch is out of bounds for the input text
    LineOutOfBounds {
        /// The line number that was out of bounds
        line: u64,
        /// The total number of lines in the input text
        total_lines: usize,
    },
    /// The context lines in the patch don't match the input text
    ContextMismatch {
        /// The line number where the mismatch occurred
        line: u64,
        /// The expected context line from the patch
        expected: String,
        /// The actual line from the input text
        actual: String,
    },
    /// The expected block of lines was not found in the input text
    HunkNotFound {
        /// The expected block of lines
        expected: String,
        /// The actual block of lines
        file_contents: String,
    },
}

impl fmt::Display for ApplyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApplyError::LineOutOfBounds { line, total_lines } => {
                write!(
                    f,
                    "Line {} is out of bounds (file has {} lines)",
                    line, total_lines
                )
            }
            ApplyError::ContextMismatch {
                line,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "Context mismatch at line {}: expected '{}', got '{}'",
                    line, expected, actual
                )
            }
            ApplyError::HunkNotFound { expected, file_contents } => {
                write!(
                    f,
                    "Hunk not found: expected to find '{}', file has '{}'",
                    expected, file_contents
                )
            }
        }
    }
}

impl Error for ApplyError {}

/// Apply a patch to the given text content
///
/// # Arguments
///
/// * `patch` - The patch to apply
/// * `content` - The text content to apply the patch to
///
/// # Returns
///
/// The patched text content if successful, or an error if the patch cannot be applied
///
/// # Example
///
/// ```
/// use patch::{Patch, apply};
///
/// let content = "line 1\nline 2\nline 3\n";
/// let patch_text = "\
/// --- old.txt
/// +++ new.txt
/// @@ -1,3 +1,3 @@
///  line 1
/// -line 2
/// +new line 2
///  line 3
/// ";
///
/// let patch = Patch::from_single(patch_text).unwrap();
/// let result = apply(&patch, content).unwrap();
/// assert_eq!(result, "line 1\nnew line 2\nline 3\n");
/// ```
pub fn apply(patch: &Patch, content: &str) -> Result<String, ApplyError> {
    let lines: Vec<&str> = content.lines().collect();
    let mut result = Vec::new();
    let mut current_line = 0;

    for hunk in &patch.hunks {
        // Add unchanged lines before the hunk
        let start = if hunk.old_range.start > 0 {
            hunk.old_range.start - 1
        } else {
            0
        };

        while current_line < start as usize {
            if current_line >= lines.len() {
                return Err(ApplyError::LineOutOfBounds {
                    line: current_line as u64 + 1,
                    total_lines: lines.len(),
                });
            }
            result.push(lines[current_line].to_string());
            current_line += 1;
        }

        let mut hunk_old_line = current_line;
        for line in &hunk.lines {
            match line {
                Line::Context(text) => {
                    if hunk_old_line >= lines.len() {
                        return Err(ApplyError::LineOutOfBounds {
                            line: hunk_old_line as u64 + 1,
                            total_lines: lines.len(),
                        });
                    }
                    if lines[hunk_old_line] != *text {
                        return Err(ApplyError::ContextMismatch {
                            line: hunk_old_line as u64 + 1,
                            expected: text.to_string(),
                            actual: lines[hunk_old_line].to_string(),
                        });
                    }
                    result.push(text.to_string());
                    hunk_old_line += 1;
                }
                Line::Add(text) => {
                    result.push(text.to_string());
                }
                Line::Remove(text) => {
                    if hunk_old_line >= lines.len() {
                        return Err(ApplyError::LineOutOfBounds {
                            line: hunk_old_line as u64 + 1,
                            total_lines: lines.len(),
                        });
                    }
                    if lines[hunk_old_line] != *text {
                        return Err(ApplyError::ContextMismatch {
                            line: hunk_old_line as u64 + 1,
                            expected: text.to_string(),
                            actual: lines[hunk_old_line].to_string(),
                        });
                    }
                    hunk_old_line += 1;
                }
            }
        }
        current_line = hunk_old_line;
    }

    // Add any remaining lines after the last hunk
    while current_line < lines.len() {
        result.push(lines[current_line].to_string());
        current_line += 1;
    }

    // Handle the end newline based on the patch's end_newline flag
    let mut output = result.join("\n");
    if !output.is_empty() && patch.end_newline {
        output.push('\n');
    }

    Ok(output)
}

/// Applies a patch to content using a find-and-replace strategy.
///
/// Unlike the standard `apply` function, this method doesn't rely on exact line numbers.
/// Instead, it searches for blocks of context and removed lines that match the patch,
/// and replaces them with the corresponding new lines.
///
/// # Arguments
/// * `patch` - The patch to apply
/// * `content` - The content to apply the patch to
///
/// # Returns
/// * `Ok(String)` - The patched content
/// * `Err(ApplyError)` - If the patch couldn't be applied
pub fn find_replace_apply(patch: &Patch, content: &str) -> Result<String, ApplyError> {
    // Split the content into lines.
    let mut content_lines: Vec<&str> = content.lines().collect();

    // Process each hunk in the patch.
    for hunk in &patch.hunks {
        // Gather the "old" lines: context and removed lines.
        let old_lines: Vec<&str> = hunk
            .lines
            .iter()
            .filter_map(|line| match line {
                Line::Context(text) | Line::Remove(text) => Some(*text),
                _ => None,
            })
            .collect();

        // Gather the "new" lines: context and added lines.
        let new_lines: Vec<&str> = hunk
            .lines
            .iter()
            .filter_map(|line| match line {
                Line::Context(text) | Line::Add(text) => Some(*text),
                _ => None,
            })
            .collect();

        // Find the occurrence of old_lines in content_lines that is closest to hunk.old_range.start.
        let mut best_index: Option<usize> = None;
        let mut best_distance: Option<usize> = None;
        // Here we assume hunk.old_range.start is a 0-indexed line number.
        let target_index = hunk.old_range.start;

        for i in 0..=content_lines.len().saturating_sub(old_lines.len()) {
            // Skip if there aren't enough lines left to match the pattern
            if i + old_lines.len() > content_lines.len() {
                
                continue;
            }
            
            if content_lines[i..i + old_lines.len()] == old_lines[..] {
                let distance = if i >= target_index as usize {
                    i - target_index as usize
                } else {
                    target_index as usize - i
                };
                if best_distance.is_none() || distance < best_distance.unwrap() {
                    best_distance = Some(distance);
                    best_index = Some(i);
                }
            }
        }

        // If we found an occurrence, replace that block of lines.
        if let Some(index) = best_index {
            content_lines.splice(index..index + old_lines.len(), new_lines.iter().cloned());
        } else {
            // If the expected block is not found, return an error.
            return Err(ApplyError::HunkNotFound {
                expected: old_lines.join("\n"),
                file_contents: content_lines.join("\n"),
            });
        }
    }

    // Join the updated lines into a single string.
    let new_content = content_lines.join("\n");
    Ok(new_content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{File, Hunk, Line, Patch, Range};
    use std::borrow::Cow;
    // Test 1: A simple replacement of a single line.
    #[test]
    fn test_simple_replace() {
        let content = "line1\nline2\nline3";
        let patch = Patch {
            old: File {
                path: Cow::Borrowed(""),
                meta: None,
            },
            new: File {
                path: Cow::Borrowed(""),
                meta: None,
            },
            end_newline: true,
            hunks: vec![Hunk {
                old_range: Range { start: 1, count: 1 },
                new_range: Range { start: 1, count: 1 },
                range_hint: "",
                lines: vec![
                    // In the patch, we expect to remove "line2" and replace it.
                    Line::Remove("line2"),
                    Line::Add("line2 modified"),
                ],
            }],
        };

        let result = find_replace_apply(&patch, content);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "line1\nline2 modified\nline3".to_string());
    }

    // Test 2: When the content contains multiple occurrences of the target block,
    // the hunk should be applied to the occurrence closest to the specified starting index.
    #[test]
    fn test_multiple_occurrences_choose_closest() {
        let content = "line1\nline2\nline3\nline2\nline3";
        let patch = Patch {
            old: File {
                path: Cow::Borrowed(""),
                meta: None,
            },
            new: File {
                path: Cow::Borrowed(""),
                meta: None,
            },
            end_newline: true,
            hunks: vec![Hunk {
                // The target index is 1.
                old_range: Range { start: 1, count: 2 },
                new_range: Range { start: 1, count: 2 },
                range_hint: "",
                lines: vec![
                    // Old lines to match: "line2" followed by "line3"
                    Line::Remove("line2"),
                    Line::Remove("line3"),
                    // New lines to replace with.
                    Line::Add("new2"),
                    Line::Add("new3"),
                ],
            }],
        };

        let result = find_replace_apply(&patch, content);
        assert!(result.is_ok());
        let expected = "line1\nnew2\nnew3\nline2\nline3".to_string();
        assert_eq!(result.unwrap(), expected);
    }

    // Test 3: When no matching block is found, the function should return an error.
    #[test]
    fn test_hunk_not_found_error() {
        let content = "line1\nline2\nline3";
        let patch = Patch {
            old: File {
                path: Cow::Borrowed(""),
                meta: None,
            },
            new: File {
                path: Cow::Borrowed(""),
                meta: None,
            },
            end_newline: true,
            hunks: vec![Hunk {
                old_range: Range { start: 1, count: 1 },
                new_range: Range { start: 1, count: 1 },
                range_hint: "",
                lines: vec![
                    // This hunk expects a block that doesn't exist in the content.
                    Line::Remove("lineX"),
                    Line::Add("lineX modified"),
                ],
            }],
        };

        let result = find_replace_apply(&patch, content);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ApplyError::HunkNotFound {
                expected: expected,
                file_contents: file_contents,
            }
        ));
    }

    // Test 4: Applying a hunk that includes context lines.
    #[test]
    fn test_context_lines() {
        let content = "line1\nline2\nline3\nline4";
        let patch = Patch {
            old: File {
                path: Cow::Borrowed(""),
                meta: None,
            },
            new: File {
                path: Cow::Borrowed(""),
                meta: None,
            },
            end_newline: true,
            hunks: vec![Hunk {
                old_range: Range { start: 1, count: 2 },
                new_range: Range { start: 1, count: 2 },
                range_hint: "",
                lines: vec![
                    // The context line ("line2") should appear in both old and new lines.
                    Line::Context("line2"),
                    // "line3" is to be removed and replaced.
                    Line::Remove("line3"),
                    Line::Add("line3 modified"),
                ],
            }],
        };

        let result = find_replace_apply(&patch, content);
        assert!(result.is_ok());
        let expected = "line1\nline2\nline3 modified\nline4".to_string();
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_apply_simple_patch() {
        let content = "line 1\nline 2\nline 3\n";
        let patch_text = "\
--- old.txt
+++ new.txt
@@ -1,3 +1,3 @@
 line 1
-line 2
+new line 2
 line 3
";
        let patch = Patch::from_single(patch_text).unwrap();
        let result = apply(&patch, content).unwrap();
        assert_eq!(result, "line 1\nnew line 2\nline 3\n");
    }

    #[test]
    fn test_apply_patch_with_additions() {
        let content = "A\nB\nC\n";
        let patch_text = "\
--- old.txt
+++ new.txt
@@ -1,3 +1,5 @@
 A
+X
 B
+Y
 C
";
        let patch = Patch::from_single(patch_text).unwrap();
        let result = apply(&patch, content).unwrap();
        assert_eq!(result, "A\nX\nB\nY\nC\n");
    }

    #[test]
    fn test_apply_patch_with_removals() {
        let content = "A\nB\nC\nD\n";
        let patch_text = "\
--- old.txt
+++ new.txt
@@ -1,4 +1,2 @@
 A
-B
-C
 D
";
        let patch = Patch::from_single(patch_text).unwrap();
        let result = apply(&patch, content).unwrap();
        assert_eq!(result, "A\nD\n");
    }

    #[test]
    fn test_apply_patch_line_out_of_bounds() {
        let content = "A\nB\n";
        let patch_text = "\
--- old.txt
+++ new.txt
@@ -1,3 +1,3 @@
 A
 B
-C
+D
";
        let patch = Patch::from_single(patch_text).unwrap();
        let err = apply(&patch, content).unwrap_err();
        match err {
            ApplyError::LineOutOfBounds { line, total_lines } => {
                assert_eq!(line, 3);
                assert_eq!(total_lines, 2);
            }
            _ => panic!("Expected LineOutOfBounds error"),
        }
    }

    #[test]
    fn test_apply_patch_context_mismatch() {
        let content = "A\nB\nC\n";
        let patch_text = "\
--- old.txt
+++ new.txt
@@ -1,3 +1,3 @@
 A
-X
+Y
 C
";
        let patch = Patch::from_single(patch_text).unwrap();
        let err = apply(&patch, content).unwrap_err();
        match err {
            ApplyError::ContextMismatch {
                line,
                expected,
                actual,
            } => {
                assert_eq!(line, 2);
                assert_eq!(expected, "X");
                assert_eq!(actual, "B");
            }
            _ => panic!("Expected ContextMismatch error"),
        }
    }
}
