use similar::{ChangeTag, TextDiff};

/// 单行 diff 结果。
#[derive(Debug, Clone)]
pub struct DiffLine {
    /// 变更类型：unchanged / insert / delete
    pub tag: DiffTag,
    /// 旧文件行号（1-based，删除行有值）
    pub old_line_num: Option<usize>,
    /// 新文件行号（1-based，新增行有值）
    pub new_line_num: Option<usize>,
    /// 行内容
    pub content: String,
    /// 单词级 diff（仅在连续的 remove+add 行对中计算）
    pub word_diffs: Option<Vec<WordDiff>>,
}

/// 单词级 diff 片段。
#[derive(Debug, Clone)]
pub struct WordDiff {
    pub tag: DiffTag,
    pub text: String,
}

/// Diff 行类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffTag {
    Unchanged,
    Insert,
    Delete,
}

/// 计算两个文本的行级 diff。
///
/// 返回按顺序排列的 DiffLine 列表。
pub fn compute_diff(old: &str, new: &str) -> Vec<DiffLine> {
    let diff = TextDiff::from_lines(old, new);
    let mut lines = Vec::new();

    for change in diff.iter_all_changes() {
        let tag = match change.tag() {
            ChangeTag::Equal => DiffTag::Unchanged,
            ChangeTag::Insert => DiffTag::Insert,
            ChangeTag::Delete => DiffTag::Delete,
        };

        let (old_num, new_num) = match change.tag() {
            ChangeTag::Delete => (change.old_index().map(|i| i + 1), None),
            ChangeTag::Insert => (None, change.new_index().map(|i| i + 1)),
            ChangeTag::Equal => (
                change.old_index().map(|i| i + 1),
                change.new_index().map(|i| i + 1),
            ),
        };

        lines.push(DiffLine {
            tag,
            old_line_num: old_num,
            new_line_num: new_num,
            content: change.value().to_string(),
            word_diffs: None,
        });
    }

    // 为连续的 Delete+Insert 行对计算单词级 diff
    compute_word_diffs(&mut lines);

    lines
}

/// 为连续的 Delete → Insert 行对计算单词级 diff。
///
/// 仅当变更比例不超过 40% 时计算（避免噪声）。
fn compute_word_diffs(lines: &mut [DiffLine]) {
    let mut i = 0;
    while i + 1 < lines.len() {
        if lines[i].tag == DiffTag::Delete && lines[i + 1].tag == DiffTag::Insert {
            let old_content = &lines[i].content;
            let new_content = &lines[i + 1].content;

            // 跳过差异过大的行对
            let total_len = old_content.len().max(1) + new_content.len().max(1);
            let change_ratio = if total_len > 0 {
                (old_content.len().abs_diff(new_content.len()) as f64 / total_len as f64).min(1.0)
            } else {
                0.0
            };

            if change_ratio <= 0.4 {
                let wdiff = compute_words(old_content, new_content);
                lines[i].word_diffs = Some(wdiff.clone());
                lines[i + 1].word_diffs = Some(wdiff);
            }

            i += 2;
        } else {
            i += 1;
        }
    }
}

/// 计算两个字符串的单词级 diff。
fn compute_words(old: &str, new: &str) -> Vec<WordDiff> {
    let raw_diff = TextDiff::from_words(old, new);

    raw_diff
        .iter_all_changes()
        .map(|change| {
            let tag = match change.tag() {
                ChangeTag::Equal => DiffTag::Unchanged,
                ChangeTag::Insert => DiffTag::Insert,
                ChangeTag::Delete => DiffTag::Delete,
            };
            WordDiff {
                tag,
                text: change.value().to_string(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_inputs() {
        let result = compute_diff("", "");
        assert!(result.is_empty());
    }

    #[test]
    fn test_all_added() {
        let result = compute_diff("", "hello\nworld\n");
        assert_eq!(result.len(), 2);
        assert!(result.iter().all(|l| l.tag == DiffTag::Insert));
    }

    #[test]
    fn test_all_deleted() {
        let result = compute_diff("hello\nworld\n", "");
        assert_eq!(result.len(), 2);
        assert!(result.iter().all(|l| l.tag == DiffTag::Delete));
    }

    #[test]
    fn test_mixed_changes() {
        let old = "line1\nline2\nline3\n";
        let new = "line1\nmodified\nline3\n";
        let result = compute_diff(old, new);
        // Equal(line1) + Delete(line2) + Insert(modified) + Equal(line3) = 4
        assert_eq!(result.len(), 4);
        assert_eq!(result[0].tag, DiffTag::Unchanged);
        assert_eq!(result[1].tag, DiffTag::Delete);
        assert_eq!(result[2].tag, DiffTag::Insert);
        assert_eq!(result[3].tag, DiffTag::Unchanged);
    }
}
