//! Eliza engine — a faithful port of the classic Creative Computing ELIZA
//! (modern.bas, GW-BASIC).
//!
//! Every control-flow quirk of the original program is reproduced exactly:
//!   * I$ = " " + input + "  ", apostrophe stripping with recheck
//!   * "SHUT"/"shut" (case-sensitive) detection inside that same loop
//!   * uppercase mirror IU$ for keyword matching, original case for output
//!   * repeat check against the previous processed input P$
//!   * keyword scan: first keyword in priority order, LAST occurrence position
//!   * conjugation pairs applied as one forward pass per pair, with the
//!     BASIC `L = L + LEN(R$)` + `NEXT L` (+1) skip
//!   * FOR-loop limits captured once at loop entry (GW-BASIC semantics)
//!   * per-keyword round-robin replies over S(K)..N(K)
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

// 14 (from, to) conjugation pairs, exactly as READ by BASIC lines 440-550.
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

// (start reply, count) per keyword, from DATA lines 2530-2560. Reply numbers
// are 1-based; keyword indices are 1-based, matching the BASIC's K.
const INDEX: [(usize, usize); 36] = [
    (1, 3), (4, 2), (6, 4), (6, 4), (10, 4), (14, 3), (17, 3), (20, 2), (22, 3), (25, 3),
    (28, 4), (28, 4), (32, 3), (35, 5), (40, 9), (40, 9), (40, 9), (40, 9), (40, 9), (40, 9),
    (49, 2), (51, 4), (55, 4), (59, 4), (63, 1), (63, 1), (64, 5), (69, 5), (74, 2), (76, 4),
    (80, 3), (83, 7), (90, 3), (93, 6), (99, 7), (106, 6),
];

/// One keyword's reply range: the BASIC's S(K)..N(K) plus the round-robin
/// pointer R(K) that walks first..last and wraps.
struct ReplyRange {
    first: usize,
    last: usize,
    next: usize,
}

pub struct Eliza {
    /// P$ — the previous processed input (with spaces, apostrophes stripped).
    prev: Vec<u8>,
    /// Per-keyword rotating reply range (BASIC arrays S/R/N).
    replies: [ReplyRange; 36],
}

impl Default for Eliza {
    fn default() -> Self {
        Self::new()
    }
}

impl Eliza {
    pub fn new() -> Self {
        Eliza {
            prev: Vec::new(),
            replies: INDEX.map(|(first, count)| ReplyRange {
                first,
                last: first + count - 1,
                next: first,
            }),
        }
    }

    /// One conversation turn. Mirrors BASIC lines 200-640.
    pub fn respond(&mut self, input: &str) -> Outcome {
        let Some(i) = preprocess(input) else {
            return Outcome::ShutUp;
        };
        let iu = uppercase(&i);
        if i == self.prev {
            return Outcome::Say("Please don't repeat yourself!".to_string());
        }
        // lines 365/370: K = matched keyword, or 36 when none found.
        let (k, c) = match find_keyword(&iu) {
            Some((k, t, f)) => (k, conjugate(&i, t, f)),
            None => (36, Vec::new()),
        };
        self.reply(k, &i, &c)
    }

    /// Lines 590-640: take reply number `range.next`, rotate the pointer,
    /// and append the conjugated remainder lowercased when the reply ends
    /// with "*".
    fn reply(&mut self, k: usize, i: &[u8], c: &[u8]) -> Outcome {
        let range = &mut self.replies[k - 1];
        let f = REPLIES[range.next - 1];
        range.next += 1;
        if range.next > range.last {
            range.next = range.first;
        }
        self.prev = i.to_vec();
        match f.strip_suffix('*') {
            Some(stem) => {
                let mut out: Vec<u8> = Vec::with_capacity(stem.len() + c.len());
                out.extend_from_slice(stem.as_bytes());
                out.extend(c.iter().map(|&ch| ch.to_ascii_lowercase()));
                Outcome::Say(String::from_utf8_lossy(&out).into_owned())
            }
            None => Outcome::Say(f.to_string()),
        }
    }
}

/// Lines 201-250: I$ = " " + input + "  ", all apostrophes dropped.
/// Returns None when the input contains "SHUT" or "shut".
fn preprocess(input: &str) -> Option<Vec<u8>> {
    let mut i: Vec<u8> = Vec::with_capacity(input.len() + 3);
    i.push(b' ');
    i.extend_from_slice(input.as_bytes());
    i.extend_from_slice(b"  ");

    // FOR L = 1 TO LEN(I$) — the limit is fixed at loop entry (GW-BASIC).
    let limit = i.len();
    let mut l = 0;
    while l < limit {
        // line 230: drop the apostrophe, then GOTO 230 (recheck same position)
        while l < i.len() && i[l] == b'\'' {
            i.remove(l);
        }
        // line 240: case-sensitive SHUT check
        if l + 4 <= i.len() && (&i[l..l + 4] == b"SHUT" || &i[l..l + 4] == b"shut") {
            return None;
        }
        l += 1;
    }
    Some(i)
}

/// Lines 251-255: IU$ — byte-wise ASCII uppercase mirror.
fn uppercase(i: &[u8]) -> Vec<u8> {
    i.iter().map(|&c| c.to_ascii_uppercase()).collect()
}

/// Lines 290-365: the first keyword (in priority order) that occurs anywhere
/// wins; its LAST occurrence position is kept, because the inner loop keeps
/// overwriting the match on every hit. Returns (1-based K, 0-based position,
/// keyword), or None when no keyword matches.
fn find_keyword(iu: &[u8]) -> Option<(usize, usize, &'static [u8])> {
    let mut found = None;
    for (k, kw) in KEYWORDS.iter().enumerate() {
        if found.is_some() {
            break; // line 315: already matched, stop searching
        }
        // FOR L = 1 TO LEN(IU$) - LEN(K$) + 1
        let limit = iu.len().saturating_sub(kw.len());
        for l in 0..=limit {
            if l + kw.len() <= iu.len() && &iu[l..l + kw.len()] == kw.as_bytes() {
                found = Some((k + 1, l, kw.as_bytes()));
            }
        }
    }
    found
}

/// Lines 390-558: conjugate the text after the keyword, exactly as the BASIC
/// does: one forward pass per pair, with the `L = L + LEN(R$)` + `NEXT L`
/// (+1) skip, per-pair loop limits, leading-space collapse, and "!" removal.
fn conjugate(i: &[u8], t: usize, f: &[u8]) -> Vec<u8> {
    // line 430: C$ = " " + (I$ after the keyword) + " "
    let mut c = Vec::new();
    c.push(b' ');
    c.extend_from_slice(&i[t + f.len()..]);
    c.push(b' ');

    for &(sp, rp) in &CONJ {
        let limit = c.len(); // FOR L = 1 TO LEN(C$) — fixed per pair
        let mut l = 0;
        while l < limit {
            // lines 470-500: match S$ -> replace with R$
            if l + sp.len() <= c.len() && &c[l..l + sp.len()] == sp.as_bytes() {
                c.splice(l..l + sp.len(), rp.bytes());
                l += rp.len() + 1; // line 495 + NEXT L's +1
                continue;
            }
            // lines 510-535: match R$ -> replace with S$
            if l + rp.len() <= c.len() && &c[l..l + rp.len()] == rp.as_bytes() {
                c.splice(l..l + rp.len(), sp.bytes());
                l += sp.len() + 1;
                continue;
            }
            l += 1;
        }
    }

    // line 555: at most one leading space
    if c.get(1) == Some(&b' ') {
        c.remove(0);
    }
    // lines 556-558: drop the "!" markers
    let limit = c.len();
    let mut l = 0;
    while l < limit {
        if l < c.len() && c[l] == b'!' {
            c.remove(l);
            continue;
        }
        l += 1;
    }
    c
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

/// C-ABI bridge for the wasm build (see web/). Compiled only for wasm32.
#[cfg(target_arch = "wasm32")]
mod wasm {
    use super::{Eliza, Outcome};
    use std::alloc::{alloc, dealloc, Layout};

    #[unsafe(no_mangle)]
    pub extern "C" fn eliza_new() -> *mut Eliza {
        Box::into_raw(Box::new(Eliza::new()))
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn eliza_free(e: *mut Eliza) {
        if !e.is_null() {
            unsafe { drop(Box::from_raw(e)) }
        }
    }

    /// Buffer JS writes the input into / reads the reply from.
    #[unsafe(no_mangle)]
    pub extern "C" fn eliza_alloc(len: usize) -> *mut u8 {
        if len == 0 {
            return std::ptr::null_mut();
        }
        unsafe { alloc(Layout::array::<u8>(len).unwrap()) }
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn eliza_dealloc(p: *mut u8, len: usize) {
        if !p.is_null() {
            unsafe { dealloc(p, Layout::array::<u8>(len).unwrap()) }
        }
    }

    /// Runs one conversation turn. Input bytes are read from `input`; the
    /// reply is written into `out` (truncated to `out_cap`); the byte length
    /// written is returned. "SHUT"/"shut" replies "Shut up...".
    #[unsafe(no_mangle)]
    pub extern "C" fn eliza_respond(
        e: *mut Eliza,
        input: *const u8,
        input_len: usize,
        out: *mut u8,
        out_cap: usize,
    ) -> usize {
        if e.is_null() || input.is_null() || out.is_null() {
            return 0;
        }
        let input = unsafe { std::slice::from_raw_parts(input, input_len) };
        let input = String::from_utf8_lossy(input);
        let reply = match unsafe { &mut *e }.respond(&input) {
            Outcome::Say(s) => s,
            Outcome::ShutUp => "Shut up...".to_string(),
        };
        let bytes = reply.as_bytes();
        let n = bytes.len().min(out_cap);
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), out, n);
        }
        n
    }
}
