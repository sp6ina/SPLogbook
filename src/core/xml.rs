// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Wspólne helpery XML: escapowanie encji, ich dekodowanie oraz ekstrakcja
//! zawartości prostych tagów `<tag>…</tag>`.
//!
//! Zgrupowane tutaj, aby uniknąć duplikacji w modułach ADIF, N1MM, FLRig,
//! PSK Reporter, HamQTH, QRZ, Solar i FLDIGI.

/// Escapuje znaki specjalne XML w treści tekstowej.
pub fn escape_xml(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

/// Zamienia encje XML na odpowiadające im znaki.
pub fn unescape_xml(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
}

/// Wyciąga zawartość pierwszego wystąpienia prostego tagu `<tag>…</tag>`,
/// dekodując encje XML. Zwraca `None`, gdy tag nie istnieje lub jest pusty.
pub fn extract_tag(xml: &str, tag: &str) -> Option<String> {
    let open_tag = format!("<{tag}>");
    let close_tag = format!("</{tag}>");

    let start = xml.find(&open_tag)? + open_tag.len();
    let end = xml[start..].find(&close_tag)? + start;
    let val = unescape_xml(xml[start..end].trim());
    if val.is_empty() { None } else { Some(val) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_handles_special_chars() {
        assert_eq!(escape_xml("a<b>&c\"d'e"), "a&lt;b&gt;&amp;c&quot;d&apos;e");
        assert_eq!(escape_xml("normal"), "normal");
    }

    #[test]
    fn unescape_handles_entities() {
        assert_eq!(
            unescape_xml("a&lt;b&gt;&amp;c&quot;d&apos;e&#39;f"),
            "a<b>&c\"d'e'f"
        );
    }

    #[test]
    fn extract_tag_returns_decoded_content() {
        let xml = "<foo><bar>a &amp; b</bar></foo>";
        assert_eq!(extract_tag(xml, "bar"), Some("a & b".to_string()));
        assert_eq!(extract_tag(xml, "baz"), None);
        assert_eq!(extract_tag("<foo><empty></empty></foo>", "empty"), None);
    }
}
