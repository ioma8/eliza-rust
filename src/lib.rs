//! Eliza engine — a faithful byte-for-byte port of the classic
//! Creative Computing ELIZA (modern.bas, GW-BASIC).
//!
//! Every data table and every control-flow quirk of the original BASIC
//! program is reproduced here exactly:
//!   * I$ = " " + input + "  ", apostrophe stripping with recheck
//!   * "SHUT"/"shut" (case-sensitive) detection inside that same loop
//!   * IU$ uppercase mirror (IL$ is built but never used in the BASIC)
//!   * repeat check against the previous processed input P$
//!   * keyword scan: first keyword in priority order, LAST occurrence position
//!   * conjugation pairs applied as one forward pass per pair, with the
//!     BASIC `L = L + LEN(R$)` + `NEXT L` (+1) skip
//!   * FOR-loop limits captured once at loop entry (GW-BASIC semantics)
//!   * reply selection: per-keyword round-robin pointer R(K) over S(K)..N(K)
//!   * "*"-suffixed replies append the conjugated remainder lowercased

pub enum Outcome {
    /// Print this text, then continue the conversation.
    Say(String),
    /// Input contained "SHUT"/"shut": print "Shut up..." and end.
    ShutUp,
}

const KEYWORDS: [&str; 36] = [
    "CAN YOU", "CAN I", "YOU ARE", "YOURE", "I DONT", "I FEEL",
    "WHY DONT YOU", "WHY CANT I", "ARE YOU", "I CANT", "I AM", "IM ",
    "YOU ", "I WANT", "WHAT", "HOW", "WHO", "WHERE", "WHEN", "WHY",
    "NAME", "CAUSE", "SORRY", "DREAM", "HELLO", "HI ", "MAYBE",
    " NO", "YOUR", "ALWAYS", "THINK", "ALIKE", "YES", "FRIEND",
    "COMPUTER", "NOKEYFOUND",
];

// 14 (from, to) conjugation pairs, exactly as READ by lines 440-550.
const CONJ: [(&str, &str); 14] = [
    (" ARE ", " am "),
    (" AM ", " are "),
    ("WERE ", "was "),
    ("WAS ", "were "),
    (" YOU ", " I "),
    (" I ", " you "),
    ("YOUR ", "my "),
    ("MY ", "your "),
    (" IVE ", " you've "),
    (" YOUVE ", " I've "),
    (" IM ", " you're "),
    (" YOURE ", " I'm "),
    (" ME ", " !you "),
    (" !YOU ", " me "),
];

const REPLIES: [&str; 112] = [
    "Don't you believe that I can*",
    "Perhaps you would like to be able to*",
    "You want me to be able to*",
    "Perhaps you don't want to*",
    "Do you want to be able to*",
    "What makes you think I am*",
    "Does it please you to believe I am*",
    "Perhaps you would like to be*",
    "Do you sometimes wish you were*",
    "Don't you really*",
    "Why don't you*",
    "Do you wish to be able to*",
    "Does that trouble you?",
    "Tell me more about such feelings.",
    "Do you often feel*",
    "Do you enjoy feeling*",
    "Do you really believe I don't*",
    "Perhaps in good time I will*",
    "Do you want me to*",
    "Do you think you should be able to*",
    "Why can't you*",
    "Why are you interested in whether or not I am*",
    "Would you prefer if I were not*",
    "Perhaps in your fantasies I am*",
    "How do you know you can't*",
    "Have you tried?",
    "Perhaps you can now*",
    "Did you come to me because you are*",
    "How long have you been*",
    "Do you believe it is normal to be*",
    "Do you enjoy being*",
    "We were discussing you, not me.",
    "Oh, I*",
    "You're not really talking about me, are you?",
    "What would it mean to you if you got*",
    "Why do you want*",
    "Suppose you soon got*",
    "What if you never got*",
    "I sometimes also want*",
    "Why do you ask?",
    "Does that question interest you?",
    "What answer would please you the most?",
    "What do you think?",
    "Are such questions on your mind often?",
    "What is it that you really want to know?",
    "Have you asked anyone else?",
    "Have you asked such questions before?",
    "What else comes to mind when you ask that?",
    "Names don't interest me.",
    "I don't care about names -- please go on.",
    "Is that the real reason?",
    "Don't any other reasons come to mind?",
    "Does that reason explain anything else?",
    "What other reasons might there be?",
    "Please don't apologize!",
    "Apologies are not necessary.",
    "What feelings do you have when you apologize?",
    "Don't be so defensive!",
    "What does that dream suggest to you?",
    "Do you dream often?",
    "What persons appear in your dreams?",
    "Are you disturbed by your dreams?",
    "How do you do... please state your problem.",
    "You don't seem quite certain.",
    "Why the uncertain tone?",
    "Can't you be more positive?",
    "You aren't sure?",
    "Don't you know?",
    "Are you saying no just to be negative?",
    "You are being a bit negative.",
    "Why not?",
    "Are you sure?",
    "Why no?",
    "Why are you concerned about my*",
    "What about your own*",
    "Can you think of a specific example?",
    "When?",
    "What are you thinking of?",
    "Really, always?",
    "Do you really think so?",
    "But you are not sure you*",
    "Do you doubt you*",
    "In what way?",
    "What resemblance do you see?",
    "What does the similarity suggest to you?",
    "What other connections do you see?",
    "Could there really be some connection?",
    "How?",
    "You seem quite positive.",
    "Are you sure?",
    "I see.",
    "I understand.",
    "Why do you bring up the topic of friends?",
    "Do your friends worry you?",
    "Do your friends pick on you?",
    "Are you sure you have any friends?",
    "Do you impose on your friends?",
    "Perhaps your love for friends worries you.",
    "Do computers worry you?",
    "Are you talking about me in particular?",
    "Are you frightened by machines?",
    "Why do you mention computers?",
    "What do you think machines have to do with your problem?",
    "Don't you think computers can help people?",
    "What is it about machines that worries you?",
    "Say, do you have any psychological problems?",
    "What does that suggest to you?",
    "I see.",
    "I'm not sure I understand you fully.",
    "Come, come, elucidate your thoughts.",
    "Can you elaborate on that?",
    "That is quite interesting.",
];

// (start reply, count) per keyword, from DATA lines 2530-2560.
const INDEX: [(usize, usize); 36] = [
    (1, 3), (4, 2), (6, 4), (6, 4), (10, 4), (14, 3), (17, 3), (20, 2), (22, 3), (25, 3),
    (28, 4), (28, 4), (32, 3), (35, 5), (40, 9), (40, 9), (40, 9), (40, 9), (40, 9), (40, 9),
    (49, 2), (51, 4), (55, 4), (59, 4), (63, 1), (63, 1), (64, 5), (69, 5), (74, 2), (76, 4),
    (80, 3), (83, 7), (90, 3), (93, 6), (99, 7), (106, 6),
];

pub struct Eliza {
    /// P$ — the previous processed input (with spaces, apostrophes stripped).
    prev: Vec<u8>,
    /// R(K) — round-robin reply pointer (1-based reply number) per keyword.
    ptr: [usize; 36],
    /// S(K) — first reply number per keyword.
    start: [usize; 36],
    /// N(K) — last reply number per keyword.
    end: [usize; 36],
}

impl Eliza {
    pub fn new() -> Self {
        let mut start = [0; 36];
        let mut end = [0; 36];
        let mut ptr = [0; 36];
        for (i, &(s, count)) in INDEX.iter().enumerate() {
            start[i] = s;
            end[i] = s + count - 1;
            ptr[i] = s;
        }
        Eliza { prev: Vec::new(), ptr, start, end }
    }

    /// One conversation turn. Mirrors BASIC lines 200-640.
    pub fn respond(&mut self, input: &str) -> Outcome {
        // line 201: I$ = " " + I$ + "  "
        let mut i: Vec<u8> = Vec::with_capacity(input.len() + 3);
        i.push(b' ');
        i.extend_from_slice(input.as_bytes());
        i.push(b' ');
        i.push(b' ');

        // lines 220-250: apostrophe removal + SHUT check.
        // FOR L = 1 TO LEN(I$) — limit captured once at entry (GW-BASIC).
        let limit = i.len();
        let mut l = 1usize;
        while l <= limit {
            // line 230: drop apostrophe at L, then GOTO 230 (recheck same L)
            while l <= i.len() && i[l - 1] == b'\'' {
                i.remove(l - 1);
            }
            // line 240: case-sensitive SHUT check
            if l + 4 <= i.len() {
                let w = &i[l - 1..l + 3];
                if w == b"SHUT" || w == b"shut" {
                    return Outcome::ShutUp;
                }
            }
            l += 1;
        }

        // lines 251-255: IU$ (uppercase mirror). IL$ is built but never used.
        let mut iu: Vec<u8> = Vec::with_capacity(i.len());
        for &c in &i {
            if (b'a'..=b'z').contains(&c) {
                iu.push(c - 32);
            } else if (b'A'..=b'Z').contains(&c) {
                iu.push(c);
            } else {
                iu.push(c);
            }
        }

        // line 256: repeat check
        if i == self.prev {
            return Outcome::Say("Please don't repeat yourself!".to_string());
        }

        // lines 290-365: find keyword. First keyword (in priority order) that
        // occurs anywhere wins; its LAST occurrence position is kept (the
        // inner loop keeps overwriting T/F$ on every match).
        let mut s = 0usize; // 0 = none found
        let mut t = 0usize; // 1-based position in IU$
        let mut f: &[u8] = &[];
        for (k, kw) in KEYWORDS.iter().enumerate() {
            if s > 0 {
                continue; // line 315: already found, skip search
            }
            // FOR L = 1 TO LEN(IU$) - LEN(K$) + 1
            let bound = iu.len().saturating_sub(kw.len()) + 1;
            for l in 1..=bound {
                if l + kw.len() - 1 <= iu.len() && &iu[l - 1..l - 1 + kw.len()] == kw.as_bytes() {
                    s = k + 1;
                    t = l;
                    f = kw.as_bytes();
                }
            }
        }
        // line 365/370: K = S (found) or K = 36 (no keyword found)
        let k = if s > 0 { s } else { 36 };

        // lines 390-558: conjugate the remainder — only when a keyword matched.
        let mut c: Vec<u8> = Vec::new();
        if s > 0 {
            // line 430: C$ = " " + RIGHT$(I$, LEN(IU$) - LEN(F$) - L + 1) + " "
            let right = iu.len() - f.len() - t + 1;
            c.push(b' ');
            c.extend_from_slice(&i[i.len() - right..]);
            c.push(b' ');

            // lines 440-550: one forward pass per pair, limit captured per pair.
            for &(sp, rp) in CONJ.iter() {
                let limit = c.len();
                let mut l = 1usize;
                while l <= limit {
                    // lines 470-500: match S$ -> replace with R$
                    if l + sp.len() <= c.len() && &c[l - 1..l - 1 + sp.len()] == sp.as_bytes() {
                        c.splice(l - 1..l - 1 + sp.len(), rp.bytes());
                        // line 495 L = L + LEN(R$); line 540 NEXT L adds 1
                        l += rp.len() + 1;
                        continue;
                    }
                    // lines 510-535: match R$ -> replace with S$
                    if l + rp.len() <= c.len() && &c[l - 1..l - 1 + rp.len()] == rp.as_bytes() {
                        c.splice(l - 1..l - 1 + rp.len(), sp.bytes());
                        l += sp.len() + 1;
                        continue;
                    }
                    l += 1;
                }
            }

            // line 555: collapse double leading space
            if c.get(1) == Some(&b' ') {
                c.remove(0);
            }
            // lines 556-558: drop all '!' (recheck same position after removal)
            let limit = c.len();
            let mut l = 1usize;
            while l <= limit {
                if l <= c.len() && c[l - 1] == b'!' {
                    c.remove(l - 1);
                    continue;
                }
                l += 1;
            }
        }

        // lines 590-610: F$ = reply R(K), then rotate the pointer.
        let f = REPLIES[self.ptr[k - 1] - 1];
        self.ptr[k - 1] += 1;
        if self.ptr[k - 1] > self.end[k - 1] {
            self.ptr[k - 1] = self.start[k - 1];
        }

        // lines 620-640: plain reply, or reply + lowercased remainder.
        self.prev = i;
        if let Some(stripped) = f.strip_suffix('*') {
            let mut out: Vec<u8> = Vec::with_capacity(stripped.len() + c.len());
            out.extend_from_slice(stripped.as_bytes());
            for &ch in &c {
                if (b'A'..=b'Z').contains(&ch) {
                    out.push(ch + 32);
                } else {
                    out.push(ch);
                }
            }
            Outcome::Say(String::from_utf8_lossy(&out).into_owned())
        } else {
            Outcome::Say(f.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn say(eliza: &mut Eliza, input: &str) -> String {
        // trim trailing whitespace: C$ carries the input's trailing spaces,
        // which are invisible on screen (the original prints them too)
        match eliza.respond(input) {
            Outcome::Say(s) => s.trim_end().to_string(),
            Outcome::ShutUp => "Shut up...".to_string(),
        }
    }

    #[test]
    fn hello_and_repeat() {
        let mut e = Eliza::new();
        assert_eq!(say(&mut e, "hello"), "How do you do... please state your problem.");
        assert_eq!(say(&mut e, "hello"), "Please don't repeat yourself!");
    }

    #[test]
    fn no_keyword_found() {
        let mut e = Eliza::new();
        assert_eq!(say(&mut e, "bye"), "Say, do you have any psychological problems?");
        // same again -> repeat message, pointer untouched
        assert_eq!(say(&mut e, "bye"), "Please don't repeat yourself!");
    }

    #[test]
    fn conjugation_via_keyword_slice() {
        let mut e = Eliza::new();
        assert_eq!(say(&mut e, "i am sad"), "Did you come to me because you are sad");
    }

    #[test]
    fn reverse_conjugation_with_exclamation() {
        let mut e = Eliza::new();
        assert_eq!(say(&mut e, "i hate you"), "We were discussing you, not me.");
        assert_eq!(say(&mut e, "you hate me"), "Oh, I hate you");
    }

    #[test]
    fn apostrophe_stripped_into_keyword() {
        let mut e = Eliza::new();
        assert_eq!(say(&mut e, "i'm sad"), "Did you come to me because you are sad");
    }

    #[test]
    fn rotation() {
        let mut e = Eliza::new();
        let expects = [
            ("what", "Why do you ask?"),
            ("what is", "Does that question interest you?"),
            ("what now", "What answer would please you the most?"),
            ("what why", "What do you think?"),
            ("what when", "Are such questions on your mind often?"),
            ("what where", "What is it that you really want to know?"),
            ("what who", "Have you asked anyone else?"),
            ("what how", "Have you asked such questions before?"),
            ("what hello", "What else comes to mind when you ask that?"),
            ("what again", "Why do you ask?"), // wraps around
        ];
        for (input, want) in expects {
            assert_eq!(say(&mut e, input), want);
        }
    }

    #[test]
    fn star_reply_appends_conjugated_remainder() {
        let mut e = Eliza::new();
        assert_eq!(say(&mut e, "can you help me"), "Don't you believe that I can help you");
        assert_eq!(say(&mut e, "you are a computer"), "What makes you think I am a computer");
        assert_eq!(say(&mut e, "i want candy"), "What would it mean to you if you got candy");
        assert_eq!(say(&mut e, "i want a car"), "Why do you want a car");
        assert_eq!(say(&mut e, "i want more"), "Suppose you soon got more");
    }

    #[test]
    fn shut_variants() {
        let mut e = Eliza::new();
        assert!(matches!(e.respond("shut up"), Outcome::ShutUp));
        let mut e = Eliza::new();
        assert!(matches!(e.respond("SHUT"), Outcome::ShutUp));
        // case-sensitive like the original: mixed case and apostrophe-split
        // forms do NOT trigger the shut-down.
        let mut e = Eliza::new();
        assert_eq!(say(&mut e, "Shut"), "Say, do you have any psychological problems?");
        let mut e = Eliza::new();
        assert_eq!(say(&mut e, "s'hut"), "Say, do you have any psychological problems?");
    }

    #[test]
    fn last_occurrence_position() {
        let mut e = Eliza::new();
        // "CAN YOU" appears twice; the LAST occurrence is used, so the
        // remainder after it is empty.
        assert_eq!(say(&mut e, "can you can you"), "Don't you believe that I can");
        let mut e = Eliza::new();
        assert_eq!(say(&mut e, "hello hello"), "How do you do... please state your problem.");
    }

    #[test]
    fn keyword_priority() {
        let mut e = Eliza::new();
        // "YOU ARE" (priority 3) beats "YOU " (priority 13)
        assert_eq!(say(&mut e, "you are"), "What makes you think I am");
    }
}
