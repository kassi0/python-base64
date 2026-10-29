use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiffKind {
    Unchanged,
    Added,
    Removed,
    Modified,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffLine {
    pub kind: DiffKind,
    pub original_line_num: Option<usize>,
    pub modified_line_num: Option<usize>,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffStats {
    pub additions: usize,
    pub deletions: usize,
    pub unchanged: usize,
    pub total_lines: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffResult {
    pub ok: bool,
    pub stats: DiffStats,
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, Deserialize)]
pub struct DiffRequest {
    pub original: String,
    pub modified: String,
    #[serde(default)]
    pub ignore_case: bool,
    #[serde(default)]
    pub ignore_whitespace: bool,
}

/// Compute Longest Common Subsequence (LCS) matrix
fn compute_lcs(a: &[String], b: &[String]) -> Vec<Vec<usize>> {
    let n = a.len();
    let m = b.len();
    let mut dp = vec![vec![0; m + 1]; n + 1];

    for i in 0..n {
        for j in 0..m {
            if a[i] == b[j] {
                dp[i + 1][j + 1] = dp[i][j] + 1;
            } else {
                dp[i + 1][j + 1] = dp[i + 1][j].max(dp[i][j + 1]);
            }
        }
    }

    dp
}

/// Compare two texts and generate line-by-line diff
pub fn compare_texts(
    original: &str,
    modified: &str,
    ignore_case: bool,
    ignore_whitespace: bool,
) -> DiffResult {
    let raw_orig: Vec<&str> = original.lines().collect();
    let raw_mod: Vec<&str> = modified.lines().collect();

    let normalize = |s: &str| -> String {
        let mut res = s.to_string();
        if ignore_case {
            res = res.to_lowercase();
        }
        if ignore_whitespace {
            res = res.split_whitespace().collect::<Vec<&str>>().join(" ");
        }
        res
    };

    let norm_orig: Vec<String> = raw_orig.iter().map(|s| normalize(s)).collect();
    let norm_mod: Vec<String> = raw_mod.iter().map(|s| normalize(s)).collect();

    let dp = compute_lcs(&norm_orig, &norm_mod);

    let mut i = raw_orig.len();
    let mut j = raw_mod.len();
    let mut reversed_diff = Vec::new();

    let mut orig_line = i;
    let mut mod_line = j;

    let mut additions = 0;
    let mut deletions = 0;
    let mut unchanged = 0;

    while i > 0 || j > 0 {
        if i > 0 && j > 0 && norm_orig[i - 1] == norm_mod[j - 1] {
            reversed_diff.push(DiffLine {
                kind: DiffKind::Unchanged,
                original_line_num: Some(orig_line),
                modified_line_num: Some(mod_line),
                content: raw_mod[j - 1].to_string(),
            });
            unchanged += 1;
            i -= 1;
            j -= 1;
            orig_line -= 1;
            mod_line -= 1;
        } else if j > 0 && (i == 0 || dp[i][j - 1] >= dp[i - 1][j]) {
            reversed_diff.push(DiffLine {
                kind: DiffKind::Added,
                original_line_num: None,
                modified_line_num: Some(mod_line),
                content: raw_mod[j - 1].to_string(),
            });
            additions += 1;
            j -= 1;
            mod_line -= 1;
        } else if i > 0 && (j == 0 || dp[i][j - 1] < dp[i - 1][j]) {
            reversed_diff.push(DiffLine {
                kind: DiffKind::Removed,
                original_line_num: Some(orig_line),
                modified_line_num: None,
                content: raw_orig[i - 1].to_string(),
            });
            deletions += 1;
            i -= 1;
            orig_line -= 1;
        }
    }

    reversed_diff.reverse();

    let total_lines = reversed_diff.len();

    DiffResult {
        ok: true,
        stats: DiffStats {
            additions,
            deletions,
            unchanged,
            total_lines,
        },
        lines: reversed_diff,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_basic() {
        let text_a = "linha 1\nlinha 2\nlinha 3";
        let text_b = "linha 1\nlinha 2 modificada\nlinha 3\nlinha 4";

        let result = compare_texts(text_a, text_b, false, false);
        assert!(result.ok);
        assert_eq!(result.stats.additions, 2);
        assert_eq!(result.stats.deletions, 1);
        assert_eq!(result.stats.unchanged, 2);
    }

    #[test]
    fn test_diff_ignore_whitespace() {
        let text_a = "teste   com    espaco";
        let text_b = "teste com espaco";

        let res_exact = compare_texts(text_a, text_b, false, false);
        assert_eq!(res_exact.stats.unchanged, 0);

        let res_ignore = compare_texts(text_a, text_b, false, true);
        assert_eq!(res_ignore.stats.unchanged, 1);
    }
}
