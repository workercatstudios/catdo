//! Text splitting shared by the per-letter text components.
//!
//! Splits text into words and spaces. Words never break inside. Japanese and Chinese may wrap
//! between any two characters, except before closing punctuation or after an opening bracket.

use unicode_properties::{GeneralCategory, UnicodeGeneralCategory as _};
use unicode_script::{Script, UnicodeScript as _};
use unicode_segmentation::UnicodeSegmentation as _;

/// A run of the split text: a word made of letters (graphemes), or whitespace.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Part {
    Word(Vec<String>),
    Space(String),
}

/// `\p{Script=Han}`, `\p{Script=Hiragana}` or `\p{Script=Katakana}`, as the web's `cjk`.
fn is_cjk(c: char) -> bool {
    matches!(
        c.script(),
        Script::Han | Script::Hiragana | Script::Katakana
    )
}

fn has_cjk(text: &str) -> bool {
    text.chars().any(is_cjk)
}

/// Opening brackets and quotes never end a line: `^[\p{Ps}\p{Pi}]`.
fn is_opening(text: &str) -> bool {
    text.chars().next().is_some_and(|c| {
        matches!(
            c.general_category(),
            GeneralCategory::OpenPunctuation | GeneralCategory::InitialPunctuation
        )
    })
}

/// Closing brackets, 、。！？ and ー never start a line: `^[\p{Pe}\p{Pf}\p{Po}ー]`.
fn is_closing(text: &str) -> bool {
    text.chars().next().is_some_and(|c| {
        c == 'ー'
            || matches!(
                c.general_category(),
                GeneralCategory::ClosePunctuation
                    | GeneralCategory::FinalPunctuation
                    | GeneralCategory::OtherPunctuation
            )
    })
}

/// Splits `text` into words of graphemes and runs of whitespace.
pub fn split(text: &str) -> Vec<Part> {
    let mut parts = Vec::new();
    for chunk in text.split_word_bounds_whitespace() {
        if chunk.chars().all(char::is_whitespace) {
            parts.push(Part::Space(chunk.to_string()));
            continue;
        }
        if !has_cjk(chunk) {
            parts.push(Part::Word(
                chunk.graphemes(true).map(str::to_string).collect(),
            ));
            continue;
        }
        let mut word: Vec<String> = Vec::new();
        for letter in chunk.graphemes(true) {
            let can_break = word.last().is_some_and(|previous| {
                (has_cjk(letter) || has_cjk(previous) || is_opening(letter))
                    && !is_closing(letter)
                    && !is_opening(previous)
            });
            if can_break {
                parts.push(Part::Word(std::mem::take(&mut word)));
            }
            word.push(letter.to_string());
        }
        if !word.is_empty() {
            parts.push(Part::Word(word));
        }
    }
    parts
}

trait SplitWhitespace {
    fn split_word_bounds_whitespace(&self) -> Vec<&str>;
}

impl SplitWhitespace for str {
    /// Splits into alternating runs of whitespace and non-whitespace.
    fn split_word_bounds_whitespace(&self) -> Vec<&str> {
        let mut runs = Vec::new();
        let mut start = 0;
        let mut space = None;
        for (index, c) in self.char_indices() {
            let is_space = c.is_whitespace();
            if space.is_some_and(|space| space != is_space) {
                runs.push(&self[start..index]);
                start = index;
            }
            space = Some(is_space);
        }
        if start < self.len() {
            runs.push(&self[start..]);
        }
        runs
    }
}

/// The order letters start in.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Order {
    /// First to last.
    #[default]
    Forward,
    /// Last to first.
    Reverse,
    /// From the middle out.
    Center,
    /// A scattered order that is the same every render: neighbours land apart.
    Shuffle,
}

/// The stagger step of each of `count` letters.
pub fn steps(count: usize, order: Order) -> Vec<usize> {
    match order {
        Order::Forward => (0..count).collect(),
        Order::Reverse => (0..count).rev().collect(),
        Order::Center => {
            let middle = (count as f32 - 1.0) / 2.0;
            (0..count)
                .map(|i| (i as f32 - middle).abs().floor() as usize)
                .collect()
        }
        Order::Shuffle => {
            // Rank by the golden-ratio sequence.
            let mut index: Vec<usize> = (0..count).collect();
            let key = |i: usize| (i as f32 * 0.618_034).fract();
            index.sort_by(|a, b| key(*a).total_cmp(&key(*b)));
            let mut step = vec![0; count];
            for (rank, i) in index.into_iter().enumerate() {
                step[i] = rank;
            }
            step
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(text: &str) -> Vec<String> {
        split(text)
            .into_iter()
            .map(|part| match part {
                Part::Word(letters) => letters.concat(),
                Part::Space(space) => space,
            })
            .collect()
    }

    #[test]
    fn latin_text_splits_on_spaces() {
        assert_eq!(
            words("Hello, pop  world"),
            ["Hello,", " ", "pop", "  ", "world"]
        );
    }

    #[test]
    fn japanese_breaks_between_letters_but_not_before_closing_marks() {
        assert_eq!(words("きらきら。"), ["き", "ら", "き", "ら。"]);
        assert_eq!(words("「ねこ」"), ["「ね", "こ」"]);
    }

    #[test]
    fn break_rules_follow_the_unicode_categories_like_the_web() {
        // Fullwidth colon and semicolon are Po: they stay with the letter before.
        assert_eq!(words("時間：三分"), ["時", "間：", "三", "分"]);
        assert_eq!(words("一；二"), ["一；", "二"]);
        // More closing marks (Pe, Pf, Po) never start a line.
        for closing in ['‥', '〟', '｝', '〙', '〗', '»', '’'] {
            let expected = [format!("ね{closing}"), "こ".to_string()];
            assert_eq!(words(&format!("ね{closing}こ")), expected);
        }
        // More opening marks (Ps, Pi) never end one.
        for opening in ['｛', '〘', '〖', '‹', '„', '〝', '«'] {
            let expected = ["ね".to_string(), format!("{opening}こ")];
            assert_eq!(words(&format!("ね{opening}こ")), expected);
        }
        // 々, 〆 and 〇 are Han: a line may break around them.
        assert_eq!(words("人々"), ["人", "々"]);
        assert_eq!(words("〆切"), ["〆", "切"]);
        assert_eq!(words("二〇"), ["二", "〇"]);
        // 〜 is a dash (Pd), so a line may break before it; not before ー.
        assert_eq!(words("ね〜"), ["ね", "〜"]);
        assert_eq!(words("ねー"), ["ねー"]);
    }

    #[test]
    fn graphemes_stay_whole() {
        let parts = split("e\u{301}!");
        assert_eq!(parts, [Part::Word(vec!["e\u{301}".into(), "!".into()])]);
    }

    #[test]
    fn orders_are_permutations() {
        let mut shuffled = steps(7, Order::Shuffle);
        shuffled.sort();
        assert_eq!(shuffled, (0..7).collect::<Vec<_>>());
        assert_eq!(steps(5, Order::Center), [2, 1, 0, 1, 2]);
        assert_eq!(steps(3, Order::Reverse), [2, 1, 0]);
    }
}
