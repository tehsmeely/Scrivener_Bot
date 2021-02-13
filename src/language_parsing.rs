//This tokenising is mostly taken from https://github.com/christophertrml/rs-natural
pub fn tokenise(text: &str) -> Vec<&str> {
    text.split(Splitter::is_match)
        .filter(|s| !s.is_empty())
        .map(Splitter::strip_leading_trailing_apostrophes)
        .collect()
}

struct Splitter;

impl Splitter {
    fn is_match(c: char) -> bool {
        match c {
            ' ' | ',' | '.' | '!' | '?' | ';' | '"' | ':' | '\t' | '\n' | '(' | ')'
            // single quote is specifically not included here and special-cased later
            | '*' | '-' => true,
            _ => false,
        }
    }

    fn strip_leading_trailing_apostrophes(s: &str) -> &str {
        let patt = '\'';
        let prefix_stripped: &str = match s.strip_prefix(patt) {
            Some(stripped) => stripped,
            None => s,
        };
        match prefix_stripped.strip_suffix(patt) {
            Some(stripped) => stripped,
            None => prefix_stripped,
        }
    }
}

#[cfg(test)]
mod testing {
    use crate::language_parsing::tokenise;

    #[test]
    fn basic_tokenising() {
        let input = "the cat sat on the mat";
        assert_eq!(
            tokenise(input),
            vec!["the", "cat", "sat", "on", "the", "mat"]
        );
    }

    #[test]
    fn tokenising_with_formatting() {
        let input = "the cat \"sat\" on **the** mat";
        assert_eq!(
            tokenise(input),
            vec!["the", "cat", "sat", "on", "the", "mat"]
        );
    }

    #[test]
    fn tokenising_with_apostrophes() {
        let input = "the cats, they're sat on the mat";
        assert_eq!(
            tokenise(input),
            vec!["the", "cats", "they're", "sat", "on", "the", "mat"]
        );
    }
}
