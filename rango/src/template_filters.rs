//! Django-like template filters and globals for MiniJinja.
//!
//! Registers a set of filters inspired by Django's built-in template filters
//! (`django.template.defaultfilters`) that aren't already provided by MiniJinja.
//!
//! Available in every template rendered via `rango::render()` or the admin panel:
//!
//! - `pluralize(arg="s")` — `{{ count }} item{{ count|pluralize }}` (pass `"y,ies"` for irregular plurals)
//! - `truncatewords(n)` — truncate a string to `n` words, appending `…`
//! - `yesno(yes="yes", no="no")` — render a boolean as text
//! - `intcomma` — format an integer with thousands separators (e.g. `1,234,567`)
//! - `linebreaks` — convert blank-line-separated text into `<p>`/`<br>` HTML (escapes HTML first)
//! - `timesince` — humanized "time since" a Unix timestamp (seconds)
//! - `default_if_none(fallback)` — explicit alias of MiniJinja's `default` for Django users
//!
//! Global functions:
//! - `now()` — current Unix timestamp (seconds), handy for `timesince`.

#![cfg(feature = "templates")]

use minijinja::value::Value;
use minijinja::{Environment, Error, ErrorKind};

/// Register all Rango/Django-inspired filters and globals on a MiniJinja environment.
pub fn register(env: &mut Environment) {
    env.add_filter("pluralize", pluralize);
    env.add_filter("truncatewords", truncatewords);
    env.add_filter("yesno", yesno);
    env.add_filter("intcomma", intcomma);
    env.add_filter("linebreaks", linebreaks);
    env.add_filter("timesince", timesince);
    env.add_filter("default_if_none", default_if_none);
    env.add_function("now", now);
}

/// `{{ count }} item{{ count|pluralize }}` → "1 item", "3 items".
/// Pass a custom suffix (`|pluralize("es")`) or an irregular `"singular,plural"` pair
/// (`|pluralize("y,ies")`).
fn pluralize(count: i64, arg: Option<String>) -> String {
    let (singular, plural) = match arg {
        Some(s) => match s.split_once(',') {
            Some((sing, plur)) => (sing.to_string(), plur.to_string()),
            None => (String::new(), s),
        },
        None => (String::new(), "s".to_string()),
    };
    if count == 1 {
        singular
    } else {
        plural
    }
}

fn truncatewords(value: String, n: u32) -> String {
    let words: Vec<&str> = value.split_whitespace().collect();
    if words.len() as u32 <= n {
        return value;
    }
    let truncated: Vec<&str> = words.into_iter().take(n as usize).collect();
    format!("{}\u{2026}", truncated.join(" "))
}

/// `{{ is_published|yesno }}` → "yes"/"no". Custom labels: `{{ is_published|yesno("Live", "Draft") }}`.
fn yesno(value: bool, yes: Option<String>, no: Option<String>) -> String {
    if value {
        yes.unwrap_or_else(|| "yes".to_string())
    } else {
        no.unwrap_or_else(|| "no".to_string())
    }
}

/// `{{ 1234567|intcomma }}` → "1,234,567".
fn intcomma(value: Value) -> Result<String, Error> {
    let s = value.to_string();
    let negative = s.starts_with('-');
    let unsigned = s.trim_start_matches('-');
    let digits: String = unsigned
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    let rest: String = unsigned.chars().skip(digits.len()).collect();

    if digits.is_empty() {
        return Err(Error::new(
            ErrorKind::InvalidOperation,
            "intcomma expects a number",
        ));
    }

    let mut grouped = String::new();
    for (i, ch) in digits.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(ch);
    }
    let grouped: String = grouped.chars().rev().collect();

    Ok(format!(
        "{}{}{}",
        if negative { "-" } else { "" },
        grouped,
        rest
    ))
}

/// Convert blank-line-separated plain text into HTML paragraphs, like Django's `linebreaks`.
/// The input is HTML-escaped first, so it's always safe to use on user-supplied text.
fn linebreaks(value: String) -> String {
    let escaped = value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;");
    let paragraphs: Vec<String> = escaped
        .split("\n\n")
        .filter(|p| !p.trim().is_empty())
        .map(|p| format!("<p>{}</p>", p.replace('\n', "<br>\n")))
        .collect();
    paragraphs.join("\n")
}

/// Humanized "time since" a Unix timestamp (seconds since epoch), like Django's `timesince`.
fn timesince(timestamp: i64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(timestamp);
    let secs = (now - timestamp).max(0);

    if secs < 60 {
        return format!("{} second{}", secs, if secs == 1 { "" } else { "s" });
    }
    let mins = secs / 60;
    if mins < 60 {
        return format!("{} minute{}", mins, if mins == 1 { "" } else { "s" });
    }
    let hours = mins / 60;
    if hours < 24 {
        return format!("{} hour{}", hours, if hours == 1 { "" } else { "s" });
    }
    let days = hours / 24;
    if days < 30 {
        return format!("{} day{}", days, if days == 1 { "" } else { "s" });
    }
    let months = days / 30;
    if months < 12 {
        return format!("{} month{}", months, if months == 1 { "" } else { "s" });
    }
    let years = months / 12;
    format!("{} year{}", years, if years == 1 { "" } else { "s" })
}

/// Explicit alias of MiniJinja's built-in `default` filter, for developers used to Django's naming.
fn default_if_none(value: Value, fallback: Value) -> Value {
    if value.is_none() || value.is_undefined() {
        fallback
    } else {
        value
    }
}

/// Current Unix timestamp in seconds — usable as `{{ now()|timesince }}`.
fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
