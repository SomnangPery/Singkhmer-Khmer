//! Many short strings kept end to end in one buffer, each found by its number.
//!
//! The dictionary has about 120,000 words and 430,000 spellings. As one
//! `String` each (24 bytes plus a separate allocation), they took over 100 MB,
//! more than an iPhone keyboard is allowed to use. Kept like this they take a
//! few bytes of overhead each.

#[derive(Default)]
pub(crate) struct Strings {
    text: String,
    ends: Vec<u32>,
}

impl Strings {
    /// Adds `s` and returns its number.
    pub fn push(&mut self, s: &str) -> u32 {
        self.text.push_str(s);
        self.ends.push(self.text.len() as u32);
        (self.ends.len() - 1) as u32
    }

    pub fn get(&self, number: u32) -> &str {
        let number = number as usize;
        let start = if number == 0 { 0 } else { self.ends[number - 1] as usize };
        &self.text[start..self.ends[number] as usize]
    }

    /// The number of `key`, when the strings were pushed in sorted order.
    pub fn find_sorted(&self, key: &str) -> Option<u32> {
        let (mut low, mut high) = (0, self.ends.len() as u32);
        while low < high {
            let middle = low + (high - low) / 2;
            match self.get(middle).cmp(key) {
                std::cmp::Ordering::Less => low = middle + 1,
                std::cmp::Ordering::Greater => high = middle,
                std::cmp::Ordering::Equal => return Some(middle),
            }
        }
        None
    }

    pub fn len(&self) -> usize {
        self.ends.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &str> {
        (0..self.ends.len() as u32).map(|number| self.get(number))
    }

    pub fn shrink_to_fit(&mut self) {
        self.text.shrink_to_fit();
        self.ends.shrink_to_fit();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strings_come_back_by_number() {
        let mut strings = Strings::default();
        assert_eq!(strings.push("bong"), 0);
        assert_eq!(strings.push(""), 1);
        assert_eq!(strings.push("បង"), 2);
        assert_eq!((strings.get(0), strings.get(1), strings.get(2)), ("bong", "", "បង"));
        assert_eq!(strings.len(), 3);
        assert_eq!(strings.iter().collect::<Vec<_>>(), ["bong", "", "បង"]);
    }

    #[test]
    fn sorted_strings_are_found() {
        let mut strings = Strings::default();
        for word in ["ban", "bong", "jg", "nh", "tv"] {
            strings.push(word);
        }
        assert_eq!(strings.find_sorted("ban"), Some(0));
        assert_eq!(strings.find_sorted("nh"), Some(3));
        assert_eq!(strings.find_sorted("tv"), Some(4));
        assert_eq!(strings.find_sorted("bo"), None);
        assert_eq!(Strings::default().find_sorted("x"), None);
    }
}
