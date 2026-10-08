use anyhow::{Result, bail};
use std::io::IsTerminal;
use std::sync::LazyLock;

static COLOR: LazyLock<bool> =
    LazyLock::new(|| std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none());

fn paint(code: &str, text: &str) -> String {
    if *COLOR {
        format!("\x1b[{code}m{text}\x1b[0m")
    } else {
        text.to_owned()
    }
}

pub fn bold(text: &str) -> String {
    paint("1", text)
}

pub fn green(text: &str) -> String {
    paint("32", text)
}

pub fn red(text: &str) -> String {
    paint("31", text)
}

pub fn dim(text: &str) -> String {
    paint("2", text)
}

pub fn header(text: &str) {
    println!("\n{}", paint("1;36", text));
}

pub fn info(text: &str) {
    println!("{} {text}", paint("36", "•"));
}

pub fn ok(text: &str) {
    println!("{} {text}", paint("32", "✓"));
}

pub fn warn(text: &str) {
    println!("{} {text}", paint("33", "!"));
}

pub fn error(text: &str) {
    eprintln!("{} {text}", paint("1;31", "error:"));
}

/// "1 stake", "22 stakes".
pub fn count(n: usize, noun: &str) -> String {
    if n == 1 {
        format!("1 {noun}")
    } else {
        format!("{n} {noun}s")
    }
}

/// One line of a transaction preview: who gains or loses how much.
pub fn change_line(who: &str, amount: i128, symbol: &str) {
    let sign = if amount > 0 { "+" } else { "" };
    let text = format!("{sign}{} {symbol}", crate::amount::format(amount));
    let text = if amount < 0 { red(&text) } else { green(&text) };
    println!("    {who:<30} {text}");
}

/// `0x2388499555aab964…f79b` style shortening for tables.
pub fn short(address: &str) -> String {
    if address.len() <= 14 {
        return address.to_owned();
    }
    format!("{}…{}", &address[..8], &address[address.len() - 4..])
}

/// Yes/no question that defaults to "no"; Esc also answers "no".
pub fn confirm(question: &str) -> Result<bool> {
    if !std::io::stdin().is_terminal() {
        bail!("refusing to sign without confirmation in a non-interactive shell; pass --yes");
    }
    match inquire::Confirm::new(question).with_default(false).prompt() {
        Ok(answer) => Ok(answer),
        Err(inquire::InquireError::OperationCanceled) => Ok(false),
        Err(inquire::InquireError::OperationInterrupted) => std::process::exit(130),
        Err(e) => Err(e.into()),
    }
}
