use op_dom::DocumentMode;

use crate::Doctype;

const QUIRKS_PUBLIC_EXACT: [&str; 3] = [
    "-//W3O//DTD W3 HTML Strict 3.0//EN//",
    "-/W3C/DTD HTML 4.0 Transitional/EN",
    "HTML",
];

const QUIRKS_PUBLIC_PREFIXES: [&str; 55] = [
    "+//Silmaril//dtd html Pro v0r11 19970101//",
    "-//AS//DTD HTML 3.0 asWedit + extensions//",
    "-//AdvaSoft Ltd//DTD HTML 3.0 asWedit + extensions//",
    "-//IETF//DTD HTML 2.0 Level 1//",
    "-//IETF//DTD HTML 2.0 Level 2//",
    "-//IETF//DTD HTML 2.0 Strict Level 1//",
    "-//IETF//DTD HTML 2.0 Strict Level 2//",
    "-//IETF//DTD HTML 2.0 Strict//",
    "-//IETF//DTD HTML 2.0//",
    "-//IETF//DTD HTML 2.1E//",
    "-//IETF//DTD HTML 3.0//",
    "-//IETF//DTD HTML 3.2 Final//",
    "-//IETF//DTD HTML 3.2//",
    "-//IETF//DTD HTML 3//",
    "-//IETF//DTD HTML Level 0//",
    "-//IETF//DTD HTML Level 1//",
    "-//IETF//DTD HTML Level 2//",
    "-//IETF//DTD HTML Level 3//",
    "-//IETF//DTD HTML Strict Level 0//",
    "-//IETF//DTD HTML Strict Level 1//",
    "-//IETF//DTD HTML Strict Level 2//",
    "-//IETF//DTD HTML Strict Level 3//",
    "-//IETF//DTD HTML Strict//",
    "-//IETF//DTD HTML//",
    "-//Metrius//DTD Metrius Presentational//",
    "-//Microsoft//DTD Internet Explorer 2.0 HTML Strict//",
    "-//Microsoft//DTD Internet Explorer 2.0 HTML//",
    "-//Microsoft//DTD Internet Explorer 2.0 Tables//",
    "-//Microsoft//DTD Internet Explorer 3.0 HTML Strict//",
    "-//Microsoft//DTD Internet Explorer 3.0 HTML//",
    "-//Microsoft//DTD Internet Explorer 3.0 Tables//",
    "-//Netscape Comm. Corp.//DTD HTML//",
    "-//Netscape Comm. Corp.//DTD Strict HTML//",
    "-//O'Reilly and Associates//DTD HTML 2.0//",
    "-//O'Reilly and Associates//DTD HTML Extended 1.0//",
    "-//O'Reilly and Associates//DTD HTML Extended Relaxed 1.0//",
    "-//SQ//DTD HTML 2.0 HoTMetaL + extensions//",
    "-//SoftQuad Software//DTD HoTMetaL PRO 6.0::19990601::extensions to HTML 4.0//",
    "-//SoftQuad//DTD HoTMetaL PRO 4.0::19971010::extensions to HTML 4.0//",
    "-//Spyglass//DTD HTML 2.0 Extended//",
    "-//Sun Microsystems Corp.//DTD HotJava HTML//",
    "-//Sun Microsystems Corp.//DTD HotJava Strict HTML//",
    "-//W3C//DTD HTML 3 1995-03-24//",
    "-//W3C//DTD HTML 3.2 Draft//",
    "-//W3C//DTD HTML 3.2 Final//",
    "-//W3C//DTD HTML 3.2//",
    "-//W3C//DTD HTML 3.2S Draft//",
    "-//W3C//DTD HTML 4.0 Frameset//",
    "-//W3C//DTD HTML 4.0 Transitional//",
    "-//W3C//DTD HTML Experimental 19960712//",
    "-//W3C//DTD HTML Experimental 970421//",
    "-//W3C//DTD W3 HTML//",
    "-//W3O//DTD W3 HTML 3.0//",
    "-//WebTechs//DTD Mozilla HTML 2.0//",
    "-//WebTechs//DTD Mozilla HTML//",
];

const HTML401_FRAMESET: &str = "-//W3C//DTD HTML 4.01 Frameset//";
const HTML401_TRANSITIONAL: &str = "-//W3C//DTD HTML 4.01 Transitional//";
const LIMITED_PUBLIC_PREFIXES: [&str; 2] = [
    "-//W3C//DTD XHTML 1.0 Frameset//",
    "-//W3C//DTD XHTML 1.0 Transitional//",
];
const QUIRKS_SYSTEM_EXACT: &str = "http://www.ibm.com/data/dtd/v11/ibmxhtml1-transitional.dtd";

pub(super) fn classify(doctype: &Doctype) -> DocumentMode {
    if doctype.force_quirks || doctype.name.as_deref() != Some("html") {
        return DocumentMode::Quirks;
    }

    let public = doctype.public_identifier.as_deref();
    let system = doctype.system_identifier.as_deref();

    if public.is_some_and(|value| {
        QUIRKS_PUBLIC_EXACT
            .iter()
            .any(|candidate| value.eq_ignore_ascii_case(candidate))
            || QUIRKS_PUBLIC_PREFIXES
                .iter()
                .any(|prefix| ascii_starts_with(value, prefix))
    }) || system.is_some_and(|value| value.eq_ignore_ascii_case(QUIRKS_SYSTEM_EXACT))
    {
        return DocumentMode::Quirks;
    }

    if let Some(public) = public {
        let html401 = ascii_starts_with(public, HTML401_FRAMESET)
            || ascii_starts_with(public, HTML401_TRANSITIONAL);
        if html401 && system.is_none_or(str::is_empty) {
            return DocumentMode::Quirks;
        }

        if LIMITED_PUBLIC_PREFIXES
            .iter()
            .any(|prefix| ascii_starts_with(public, prefix))
            || (html401 && system.is_some_and(|value| !value.is_empty()))
        {
            return DocumentMode::LimitedQuirks;
        }
    }

    DocumentMode::NoQuirks
}

fn ascii_starts_with(value: &str, prefix: &str) -> bool {
    value
        .get(..prefix.len())
        .is_some_and(|candidate| candidate.eq_ignore_ascii_case(prefix))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doctype(public: Option<&str>, system: Option<&str>) -> Doctype {
        Doctype {
            name: Some("html".into()),
            public_identifier: public.map(str::to_owned),
            system_identifier: system.map(str::to_owned),
            force_quirks: false,
        }
    }

    #[test]
    fn every_pinned_quirks_identifier_is_ascii_case_insensitive() {
        for exact in QUIRKS_PUBLIC_EXACT {
            assert_eq!(classify(&doctype(Some(exact), None)), DocumentMode::Quirks);
            assert_eq!(
                classify(&doctype(Some(&exact.to_ascii_lowercase()), None)),
                DocumentMode::Quirks
            );
        }
        for prefix in QUIRKS_PUBLIC_PREFIXES {
            let extended = format!("{prefix}suffix");
            assert_eq!(
                classify(&doctype(Some(&extended), None)),
                DocumentMode::Quirks,
                "{prefix}"
            );
            assert_eq!(
                classify(&doctype(Some(&extended.to_ascii_lowercase()), None)),
                DocumentMode::Quirks,
                "{prefix}"
            );
        }
        assert_eq!(
            classify(&doctype(None, Some(QUIRKS_SYSTEM_EXACT))),
            DocumentMode::Quirks
        );
    }

    #[test]
    fn html401_system_presence_splits_quirks_from_limited_quirks() {
        for public in [HTML401_FRAMESET, HTML401_TRANSITIONAL] {
            assert_eq!(classify(&doctype(Some(public), None)), DocumentMode::Quirks);
            assert_eq!(
                classify(&doctype(Some(public), Some(""))),
                DocumentMode::Quirks
            );
            assert_eq!(
                classify(&doctype(Some(public), Some("legacy.dtd"))),
                DocumentMode::LimitedQuirks
            );
        }
    }

    #[test]
    fn xhtml_legacy_prefixes_select_limited_quirks() {
        for prefix in LIMITED_PUBLIC_PREFIXES {
            assert_eq!(
                classify(&doctype(Some(&format!("{prefix}EN")), None)),
                DocumentMode::LimitedQuirks
            );
        }
    }

    #[test]
    fn force_quirks_wrong_name_and_modern_doctypes_select_expected_modes() {
        let mut forced = doctype(None, None);
        forced.force_quirks = true;
        assert_eq!(classify(&forced), DocumentMode::Quirks);

        let mut wrong_name = doctype(None, None);
        wrong_name.name = Some("svg".into());
        assert_eq!(classify(&wrong_name), DocumentMode::Quirks);

        assert_eq!(classify(&doctype(None, None)), DocumentMode::NoQuirks);
        assert_eq!(
            classify(&doctype(None, Some("about:legacy-compat"))),
            DocumentMode::NoQuirks
        );
        assert_eq!(
            classify(&doctype(Some("-//EXAMPLE//DTD HTML Future//EN"), None)),
            DocumentMode::NoQuirks
        );
    }
}
