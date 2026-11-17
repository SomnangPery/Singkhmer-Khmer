use crate::chars::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Options {
    pub fold_coeng_da: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options { fold_coeng_da: true }
    }
}

pub fn normalize(text: &str) -> String {
    normalize_with(text, Options::default())
}

pub fn normalize_with(text: &str, options: Options) -> String {
    let chars = prepass(text);
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if !is_base(c) {
            out.push(c);
            i += 1;
            if c == COENG && i < chars.len() && is_base(chars[i]) {
                out.push(chars[i]);
                i += 1;
            }
            continue;
        }
        let (mut syllable, end) = Syllable::read(&chars, i);
        syllable.fix(options);
        syllable.write(&mut out);
        i = end;
    }
    out
}

fn prepass(text: &str) -> Vec<char> {
    let mut out = Vec::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\u{17A3}' => out.push('\u{17A2}'),
            '\u{17A4}' => {
                out.push('\u{17A2}');
                out.push(AA);
            }
            COENG if out.last() == Some(&COENG) => {}
            _ => out.push(c),
        }
    }
    out
}

#[derive(Default)]
struct Syllable {
    base: char,
    robats: Vec<char>,
    coengs: Vec<char>,
    shifters: Vec<char>,
    zwnj: usize,
    pre: Vec<char>,
    below: Vec<char>,
    above: Vec<char>,
    post: Vec<char>,
    signs: Vec<char>,
    finals: Vec<char>,
}

impl Syllable {
    fn read(chars: &[char], start: usize) -> (Syllable, usize) {
        let mut s = Syllable {
            base: chars[start],
            ..Syllable::default()
        };
        let mut j = start + 1;
        while j < chars.len() {
            let c = chars[j];
            let next = chars.get(j + 1).copied();
            match class_of(c) {
                Class::Base => break,
                Class::Coeng => match next {
                    Some(sub) if is_base(sub) => {
                        s.coengs.push(sub);
                        j += 2;
                        continue;
                    }
                    _ => break,
                },
                Class::Other => {
                    let before_shifter = next.map_or(false, |n| class_of(n) == Class::Shifter);
                    let after_shifter = !s.shifters.is_empty() && !s.has_vowel_or_sign();
                    if c == ZWNJ && (before_shifter || after_shifter) {
                        s.zwnj += 1;
                    } else {
                        break;
                    }
                }
                Class::Robat => s.robats.push(c),
                Class::Shifter => s.shifters.push(c),
                Class::VowelPre => s.pre.push(c),
                Class::VowelBelow => s.below.push(c),
                Class::VowelAbove => s.above.push(c),
                Class::VowelPost => s.post.push(c),
                Class::Sign => s.signs.push(c),
                Class::Final => s.finals.push(c),
            }
            j += 1;
        }
        (s, j)
    }

    fn has_vowel_or_sign(&self) -> bool {
        !(self.pre.is_empty()
            && self.below.is_empty()
            && self.above.is_empty()
            && self.post.is_empty()
            && self.signs.is_empty()
            && self.finals.is_empty())
    }

    fn fix(&mut self, options: Options) {
        self.coengs.sort_by_key(|&c| c == RO);

        if options.fold_coeng_da {
            for c in &mut self.coengs {
                if *c == DA {
                    *c = TA;
                }
            }
        }

        let mut k = 0;
        while k + 1 < self.pre.len() {
            if self.pre[k] == E && self.pre[k + 1] == E {
                self.pre[k] = AE;
                self.pre.remove(k + 1);
            }
            k += 1;
        }

        if self.below.len() <= 1 {
            if let Some(last) = self.pre.last_mut() {
                if *last == E && self.above.first() == Some(&II) {
                    *last = OE;
                    self.above.remove(0);
                } else if *last == E && self.above.is_empty() && self.post.first() == Some(&AA) {
                    *last = OO;
                    self.post.remove(0);
                }
            }
        }

        if self.shifters.is_empty() && self.below == [U] && self.coengs.len() <= 2 {
            let strong = is_strong_cluster(self.base, &self.coengs);
            if self.u_is_a_downshifted_shifter(strong) {
                self.below.clear();
                self.shifters.push(if strong { TRIISAP } else { MUUSIKATOAN });
            }
        }
    }

    fn u_is_a_downshifted_shifter(&self, strong: bool) -> bool {
        if self.pre == [OE] && self.above.is_empty() {
            return true;
        }
        let pre_ok = self.pre.len() <= 1 && self.pre.iter().all(|v| ('\u{17C1}'..='\u{17C5}').contains(v));
        if !pre_ok {
            return false;
        }
        if !self.above.is_empty() {
            return true;
        }
        if self.post == [AA] {
            return self.signs.first() == Some(&NIKAHIT);
        }
        if self.post.is_empty() {
            return match self.signs.first() {
                Some(&ATTHACAN) => true,
                Some(&SAMYOK) => !strong,
                _ => false,
            };
        }
        false
    }

    fn write(&self, out: &mut String) {
        out.push(self.base);
        out.extend(&self.robats);
        for &c in &self.coengs {
            out.push(COENG);
            out.push(c);
        }
        out.extend(&self.shifters);
        for _ in 0..self.zwnj {
            out.push(ZWNJ);
        }
        out.extend(&self.pre);
        out.extend(&self.below);
        out.extend(&self.above);
        out.extend(&self.post);
        out.extend(&self.signs);
        out.extend(&self.finals);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subscripts_go_before_the_vowel() {
        assert_eq!(
            normalize("\u{1781}\u{17BB}\u{17C6}\u{17D2}\u{1789}"),
            "\u{1781}\u{17D2}\u{1789}\u{17BB}\u{17C6}"
        );
    }

    #[test]
    fn coeng_ro_goes_last() {
        assert_eq!(
            normalize("\u{179F}\u{17D2}\u{179A}\u{17D2}\u{178F}\u{17B8}"),
            "\u{179F}\u{17D2}\u{178F}\u{17D2}\u{179A}\u{17B8}"
        );
    }

    #[test]
    fn coeng_da_becomes_coeng_ta_unless_turned_off() {
        let typed = "\u{179F}\u{17D2}\u{178A}\u{17B6}";
        assert_eq!(normalize(typed), "\u{179F}\u{17D2}\u{178F}\u{17B6}");
        let keep = Options { fold_coeng_da: false };
        assert_eq!(normalize_with(typed, keep), typed);
    }

    #[test]
    fn shifter_goes_after_subscripts() {
        assert_eq!(
            normalize("\u{1798}\u{17C9}\u{17D2}\u{1799}\u{17B6}\u{1784}"),
            "\u{1798}\u{17D2}\u{1799}\u{17C9}\u{17B6}\u{1784}"
        );
    }

    #[test]
    fn robat_vowel_sign_final_order() {
        assert_eq!(normalize("\u{1780}\u{17B6}\u{17CC}"), "\u{1780}\u{17CC}\u{17B6}");
        assert_eq!(normalize("\u{1780}\u{17C6}\u{17BB}"), "\u{1780}\u{17BB}\u{17C6}");
        assert_eq!(normalize("\u{1780}\u{17C7}\u{17C6}"), "\u{1780}\u{17C6}\u{17C7}");
    }

    #[test]
    fn lookalike_vowels_are_merged() {
        assert_eq!(normalize("\u{1780}\u{17C1}\u{17B6}"), "\u{1780}\u{17C4}");
        assert_eq!(normalize("\u{1780}\u{17C1}\u{17B8}"), "\u{1780}\u{17BE}");
        assert_eq!(normalize("\u{1780}\u{17C1}\u{17C1}"), "\u{1780}\u{17C2}");
    }

    #[test]
    fn u_under_an_above_vowel_becomes_the_right_shifter() {
        assert_eq!(normalize("\u{179F}\u{17BB}\u{17B8}"), "\u{179F}\u{17CA}\u{17B8}");
        assert_eq!(normalize("\u{1798}\u{17BB}\u{17BA}"), "\u{1798}\u{17C9}\u{17BA}");
        assert_eq!(normalize("\u{1794}\u{17BB}\u{17B7}"), "\u{1794}\u{17C9}\u{17B7}");
        assert_eq!(
            normalize("\u{1789}\u{17BB}\u{17B6}\u{17C6}"),
            "\u{1789}\u{17C9}\u{17B6}\u{17C6}"
        );
    }

    #[test]
    fn a_real_u_is_left_alone() {
        assert_eq!(normalize("\u{1780}\u{17BB}"), "\u{1780}\u{17BB}");
        assert_eq!(normalize("\u{1780}\u{17BB}\u{17C6}"), "\u{1780}\u{17BB}\u{17C6}");
        assert_eq!(normalize("\u{1780}\u{17BB}\u{17B6}"), "\u{1780}\u{17BB}\u{17B6}");
    }

    #[test]
    fn deprecated_letters_are_replaced() {
        assert_eq!(normalize("\u{17A3}"), "\u{17A2}");
        assert_eq!(normalize("\u{17A4}"), "\u{17A2}\u{17B6}");
    }

    #[test]
    fn text_that_is_not_a_syllable_is_untouched() {
        for text in [
            "",
            "hello",
            "\u{17B6}\u{1780}",
            "\u{17D2}\u{1780}\u{17B6}",
            "\u{1780}\u{17D2}\u{17B6}",
        ] {
            assert_eq!(normalize(text), text);
        }
    }
}
