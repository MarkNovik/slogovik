use std::io::Read;

fn archives() -> (Vec<String>, Vec<String>) {
    const DICTS: &[u8] = include_bytes!("dicts.zip");
    let mut arch = zip::ZipArchive::new(std::io::Cursor::new(DICTS)).unwrap();
    let mut russian = String::new();
    _ = arch
        .by_name("russian.txt")
        .unwrap()
        .read_to_string(&mut russian)
        .unwrap();

    let mut ukrainian = String::new();
    _ = arch
        .by_name("ukrainian.txt")
        .unwrap()
        .read_to_string(&mut ukrainian)
        .unwrap();

    (
        russian.lines().map(str::to_string).collect(),
        ukrainian.lines().map(str::to_string).collect(),
    )
}

fn main() {
    let mut args = std::env::args();
    let _program = args.next().unwrap();
    let args = args.collect::<Vec<_>>();
    match args.first().map(|s| s.as_str()) {
        None => {
            println!("{}", usage());
            eprintln!("ERROR: No args provided");
            std::process::exit(1);
        }
        Some("repl") => {
            if args.len() > 1 {
                println!("WARN: Any arguments in repl mode are ignored");
            }
            repl();
        }

        Some("showcase") => {
            let n = match args.get(1).map(|m| m.parse::<usize>()) {
                None => 5,
                Some(Ok(n)) => n,
                Some(Err(_)) => {
                    println!("{}", usage());
                    eprintln!(
                        "ERR: Expected n to be a positive integer, got `{}`",
                        args[1]
                    );
                    std::process::exit(1);
                }
            };
            showcase(n);
        }

        Some("syl") => {
            if args.len() > 1 {
                syllabize(&args[1..])
            } else {
                println!("{}", usage());
                eprintln!("ERROR: No words provided");
                std::process::exit(1);
            }
        }

        Some("help") => {
            println!("{}", usage());
        }

        Some(_) => syllabize(&args),
    }
}

fn usage() -> String {
    use std::fmt::Write;
    let mut usage = String::new();
    let program = std::env::args()
        .next()
        .expect("No program arg was available");

    writeln!(usage, "Usage: {program} <mode> [args]").unwrap();
    writeln!(usage, "Modes:").unwrap();
    writeln!(
        usage,
        "\trepl          - REPL mode, enter a word and get it syllabalized."
    )
    .unwrap();
    writeln!(
        usage,
        "\tshowcase [n]  - showcase, prints n syllabalized russian and ukrainian words, default n = 5."
    ).unwrap();
    writeln!(
        usage,
        "\tsyl [args...] - default mode, syllabize each word in args, chosen if none specified."
    )
    .unwrap();
    writeln!(usage, "\thelp          - prints this help message.").unwrap();
    usage
}

fn maybe_rand() -> usize {
    (current_time_micros() << current_time_micros() % usize::BITS as u128 ^ !current_time_micros())
        as usize
}

fn current_time_micros() -> u128 {
    match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(t) => t.as_micros(),
        Err(_) => unsafe {
            std::hint::unreachable_unchecked(/* There is no way system clock is earlier than UNIX_EPOCH */)
        },
    }
}

fn showcase(n: usize) {
    let (russian, ukrainian) = archives();

    for _ in 0..n {
        let word = &russian[maybe_rand() % russian.len()];
        println!("{word}: {}", split_syllables(word).join("-"));
    }

    println!("---");

    for _ in 0..n {
        let word = &ukrainian[maybe_rand() % ukrainian.len()];
        println!("{word}: {}", split_syllables(word).join("-"));
    }
}

fn split_syllables(word: &str) -> Vec<String> {
    word.split(&['-', '\''])
        .map(split_syllables_pure)
        .fold(vec![], |mut acc, mut b| {
            acc.append(&mut b);
            acc
        })
}

fn split_syllables_pure(word: &str) -> Vec<String> {
    let vowels = word.match_indices(is_cyrillic_vowel).collect::<Vec<_>>();
    if vowels.len() == 1 {
        return vec![word.to_string()];
    }

    let mut sylls = vec![];
    let mut i = 0;

    for (boundary, c) in vowels {
        let to = boundary + c.len();
        sylls.push((&word[i..to]).to_string());
        i = to;
    }
    sylls.last_mut().unwrap().extend((&word[i..]).chars());

    for this_idx in 0..(sylls.len() - 1) {
        let mut this = sylls[this_idx].clone();
        let mut next = sylls[this_idx + 1].clone();

        let nv = next
            .chars()
            .position(|c| is_cyrillic_vowel(c))
            .expect("syllable without vowel");

        match nv {
            0 | 1 => (),
            nv => {
                this.extend(next.chars().take(nv - 1));
                next = next.chars().skip(nv - 1).collect();

                if next.find(is_cyrillic_sign) == Some(0) {
                    let s = next.remove(0);
                    this.push(s);
                }

                if next.find(is_apostrophe) == Some(0) {
                    let s = this.pop().unwrap();
                    next.insert(0, s);
                }

                if next.starts_with(&['Р', 'р'])
                    && count_last(this.chars(), |c| !is_cyrillic_vowel(c)) > 1
                {
                    next.insert(0, this.pop().unwrap());
                }

                sylls[this_idx] = this;
                sylls[this_idx + 1] = next;
            }
        }
    }

    sylls
}

fn is_cyrillic_vowel(c: char) -> bool {
    const VOWELS: &str = "АаЯяОоЁеЭэЕеЄєЫыИиІіЇїУуЮю";
    VOWELS.contains(c)
}

fn is_cyrillic_sign(c: char) -> bool {
    const SIGNS: &str = "ЬьЪъ";
    SIGNS.contains(c)
}

fn is_apostrophe(c: char) -> bool {
    const APOSTROPHES: &str = "'`’ʼ′";
    APOSTROPHES.contains(c)
}

fn is_cyrillic(c: &char) -> bool {
    ('\u{0400}'..='\u{04FF}').contains(c)
}

fn repl() {
    use std::io::Write;

    let mut buf = String::new();
    let stdin = std::io::stdin();

    println!("Welcome to slogovik repl.");
    println!("Enter a russian or ukrainian word to split it by syllables.");
    println!("\t:q or :ь to exit\n\t:h or :э for help.");

    loop {
        buf.clear();
        print!("slogovik> ");
        std::io::stdout().flush().unwrap();
        _ = stdin.read_line(&mut buf).unwrap();
        let word = buf.trim();

        match word {
            ":q" | ":ь" => return,
            ":h" | ":э" => {
                println!("Enter a russian or ukrainian word to split it by syllables.");
                println!("Availiable commands:");
                println!("\t:q - quit");
                println!("\t:h - this help message");
            }
            _ if word.chars().any(|c| !is_cyrillic(&c)) => {
                println!("Only cyrillic words are allowed");
                continue;
            }
            _ => {
                println!("{word}: {}", split_syllables(word).join("-"));
            }
        }
    }
}

fn syllabize(words: &[String]) {
    for word in words {
        if word.chars().any(|c| !is_cyrillic(&c)) {
            println!("`{word}` is ignored: Only cyrillic letters are allowed");
            continue;
        }
        println!("{word}: {}", split_syllables(word).join("-"));
    }
}

fn count_last<I: DoubleEndedIterator>(i: I, predicate: impl Fn(I::Item) -> bool) -> usize {
    let mut count = 0;
    for i in i.rev() {
        if predicate(i) {
            count += 1;
        } else {
            break;
        }
    }
    count
}
