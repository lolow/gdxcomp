use serde::Serialize;

/// Position of a file within a WITCH convergence set (`<core>_r<run>_i<iter>`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IterTag {
    pub run: u32,
    pub iter: u32,
}

fn token_number(token: &str, prefix: char) -> Option<u32> {
    let digits = token.strip_prefix(prefix)?;
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

/// Splits a file stem into its core name and optional `_r<n>` / `_i<n>` tokens.
fn parse_tag(stem: &str) -> (String, Option<u32>, Option<u32>) {
    let mut core: Vec<&str> = Vec::new();
    let mut run = None;
    let mut iter = None;
    for (idx, token) in stem.split('_').enumerate() {
        // The first token is always the core, so a stem like "r1" is not a tag.
        if idx > 0 {
            if let Some(n) = token_number(token, 'r') {
                run = Some(n);
                continue;
            }
            if let Some(n) = token_number(token, 'i') {
                iter = Some(n);
                continue;
            }
        }
        core.push(token);
    }
    (core.join("_"), run, iter)
}

/// Tags every stem when they form one convergence set: at least two files,
/// a shared core once the run/iteration tokens are removed, and at least one
/// `_i<n>` token. A missing run means run 1; a missing iteration means 0.
pub fn iteration_tags<S: AsRef<str>>(stems: &[S]) -> Option<Vec<IterTag>> {
    if stems.len() < 2 {
        return None;
    }
    let parsed: Vec<_> = stems.iter().map(|s| parse_tag(s.as_ref())).collect();
    let core = &parsed[0].0;
    if parsed.iter().any(|(c, _, _)| c != core) || parsed.iter().all(|(_, _, i)| i.is_none()) {
        return None;
    }
    Some(
        parsed
            .into_iter()
            .map(|(_, run, iter)| IterTag {
                run: run.unwrap_or(1),
                iter: iter.unwrap_or(0),
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tag(run: u32, iter: u32) -> IterTag {
        IterTag { run, iter }
    }

    #[test]
    fn parses_run_and_iteration() {
        assert_eq!(
            parse_tag("debug_r1_i10"),
            ("debug".to_string(), Some(1), Some(10))
        );
        assert_eq!(
            parse_tag("results_witch_i3_r2"),
            ("results_witch".to_string(), Some(2), Some(3))
        );
        assert_eq!(parse_tag("base"), ("base".to_string(), None, None));
    }

    #[test]
    fn tags_a_convergence_set() {
        assert_eq!(
            iteration_tags(&["debug_r1_i1", "debug_r1_i10", "debug_r2_i3"]),
            Some(vec![tag(1, 1), tag(1, 10), tag(2, 3)])
        );
    }

    #[test]
    fn missing_run_defaults_to_one() {
        assert_eq!(
            iteration_tags(&["foo_i3", "foo_i4"]),
            Some(vec![tag(1, 3), tag(1, 4)])
        );
    }

    #[test]
    fn mixed_cores_are_not_a_set() {
        assert_eq!(iteration_tags(&["debug_r1_i1", "other_r1_i2"]), None);
    }

    #[test]
    fn single_file_is_not_a_set() {
        assert_eq!(iteration_tags(&["debug_r1_i1"]), None);
    }

    #[test]
    fn runs_without_iterations_are_not_a_set() {
        assert_eq!(iteration_tags(&["debug_r1", "debug_r2"]), None);
    }

    #[test]
    fn non_numeric_tokens_stay_in_core() {
        assert_eq!(
            iteration_tags(&["ssp2_iam_i1", "ssp2_iam_i2"]).map(|v| v.len()),
            Some(2)
        );
        assert_eq!(parse_tag("run_ix").0, "run_ix");
    }
}
