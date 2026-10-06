use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceType {
    Document,
    Stylesheet,
    Image,
    Script,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequestDecision {
    Allow,
    Block { rule: String },
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FilterStats {
    pub checked: u64,
    pub allowed: u64,
    pub blocked: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FilterImportReport {
    pub accepted: usize,
    pub ignored: usize,
}

#[derive(Debug)]
pub struct RequestFilter {
    rules: Vec<FilterRule>,
    site_allowlist: Vec<String>,
    checked: AtomicU64,
    allowed: AtomicU64,
    blocked: AtomicU64,
}

impl Default for RequestFilter {
    fn default() -> Self {
        Self {
            rules: Vec::new(),
            site_allowlist: Vec::new(),
            checked: AtomicU64::new(0),
            allowed: AtomicU64::new(0),
            blocked: AtomicU64::new(0),
        }
    }
}

impl RequestFilter {
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    pub fn clear_rules(&mut self) {
        self.rules.clear();
    }

    pub fn allow_site(&mut self, host: impl Into<String>) {
        let host = normalize_host(&host.into());
        if !host.is_empty() && !self.site_allowlist.iter().any(|entry| entry == &host) {
            self.site_allowlist.push(host);
        }
    }

    pub fn remove_site_allow(&mut self, host: &str) {
        let host = normalize_host(host);
        self.site_allowlist.retain(|entry| entry != &host);
    }

    pub fn import_adblock_rules(&mut self, text: &str) -> FilterImportReport {
        let mut report = FilterImportReport::default();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty()
                || line.starts_with('!')
                || (line.starts_with('[') && line.ends_with(']'))
            {
                continue;
            }

            match FilterRule::parse(line) {
                Some(rule) => {
                    self.rules.push(rule);
                    report.accepted += 1;
                }
                None => report.ignored += 1,
            }
        }
        report
    }

    pub fn check(
        &self,
        url: &str,
        resource_type: ResourceType,
        top_level_url: Option<&str>,
    ) -> RequestDecision {
        self.checked.fetch_add(1, Ordering::Relaxed);

        if top_level_url
            .and_then(extract_host)
            .is_some_and(|host| self.site_is_allowed(host))
        {
            self.allowed.fetch_add(1, Ordering::Relaxed);
            return RequestDecision::Allow;
        }

        let host = extract_host(url);
        let mut block = None;
        for rule in &self.rules {
            if !rule.matches(url, host, resource_type) {
                continue;
            }
            if rule.exception {
                self.allowed.fetch_add(1, Ordering::Relaxed);
                return RequestDecision::Allow;
            }
            if block.is_none() {
                block = Some(rule.raw.clone());
            }
        }

        if let Some(rule) = block {
            self.blocked.fetch_add(1, Ordering::Relaxed);
            RequestDecision::Block { rule }
        } else {
            self.allowed.fetch_add(1, Ordering::Relaxed);
            RequestDecision::Allow
        }
    }

    pub fn stats(&self) -> FilterStats {
        FilterStats {
            checked: self.checked.load(Ordering::Relaxed),
            allowed: self.allowed.load(Ordering::Relaxed),
            blocked: self.blocked.load(Ordering::Relaxed),
        }
    }

    pub fn reset_stats(&self) {
        self.checked.store(0, Ordering::Relaxed);
        self.allowed.store(0, Ordering::Relaxed);
        self.blocked.store(0, Ordering::Relaxed);
    }

    fn site_is_allowed(&self, host: &str) -> bool {
        self.site_allowlist
            .iter()
            .any(|allowed| host_matches_suffix(host, allowed))
    }
}

#[derive(Debug)]
struct FilterRule {
    raw: String,
    exception: bool,
    pattern: RulePattern,
    resource_types: Option<Vec<ResourceType>>,
}

#[derive(Debug)]
enum RulePattern {
    HostSuffix(String),
    Wildcard(Vec<String>),
}

impl FilterRule {
    fn parse(source: &str) -> Option<Self> {
        let (exception, source) = if let Some(rest) = source.strip_prefix("@@") {
            (true, rest)
        } else {
            (false, source)
        };
        let (pattern_text, options) = source.split_once('$').unwrap_or((source, ""));
        let resource_types = parse_resource_types(options)?;

        let pattern = if let Some(host) = pattern_text
            .strip_prefix("||")
            .and_then(|value| value.strip_suffix('^'))
        {
            let host = normalize_host(host);
            if host.is_empty() {
                return None;
            }
            RulePattern::HostSuffix(host)
        } else {
            let pattern_text = pattern_text.trim_matches('|');
            let segments = pattern_text
                .split('*')
                .filter(|segment| !segment.is_empty())
                .map(|segment| segment.to_ascii_lowercase())
                .collect::<Vec<_>>();
            if segments.is_empty() {
                return None;
            }
            RulePattern::Wildcard(segments)
        };

        Some(Self {
            raw: source.to_owned(),
            exception,
            pattern,
            resource_types,
        })
    }

    fn matches(&self, url: &str, host: Option<&str>, resource_type: ResourceType) -> bool {
        if self
            .resource_types
            .as_ref()
            .is_some_and(|types| !types.contains(&resource_type))
        {
            return false;
        }

        match &self.pattern {
            RulePattern::HostSuffix(suffix) => {
                host.is_some_and(|host| host_matches_suffix(host, suffix))
            }
            RulePattern::Wildcard(segments) => wildcard_match(&url.to_ascii_lowercase(), segments),
        }
    }
}

fn parse_resource_types(options: &str) -> Option<Option<Vec<ResourceType>>> {
    if options.is_empty() {
        return Some(None);
    }

    let mut types = Vec::new();
    for option in options
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let resource = match option {
            "document" => ResourceType::Document,
            "stylesheet" | "css" => ResourceType::Stylesheet,
            "image" => ResourceType::Image,
            "script" => ResourceType::Script,
            _ => return None,
        };
        if !types.contains(&resource) {
            types.push(resource);
        }
    }
    Some(Some(types))
}

fn wildcard_match(url: &str, segments: &[String]) -> bool {
    let mut rest = url;
    for segment in segments {
        let Some(index) = rest.find(segment) else {
            return false;
        };
        rest = &rest[index + segment.len()..];
    }
    true
}

fn extract_host(url: &str) -> Option<&str> {
    let (_, rest) = url.split_once("://")?;
    let authority = rest.split(['/', '?', '#']).next()?;
    let authority = authority.rsplit('@').next().unwrap_or(authority);
    if authority.starts_with('[') {
        return authority
            .strip_prefix('[')?
            .split_once(']')
            .map(|(host, _)| host);
    }
    Some(authority.split(':').next().unwrap_or(authority))
}

fn normalize_host(host: &str) -> String {
    host.trim().trim_matches('.').to_ascii_lowercase()
}

fn host_matches_suffix(host: &str, suffix: &str) -> bool {
    let host = host.trim_matches('.').to_ascii_lowercase();
    host == suffix
        || host
            .strip_suffix(suffix)
            .is_some_and(|prefix| prefix.ends_with('.'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_host_suffixes_and_resource_specific_patterns() {
        let mut filter = RequestFilter::default();
        let report = filter
            .import_adblock_rules("||ads.example^\n*analytics*$script\n*hero*$image\n! comment");
        assert_eq!(report.accepted, 3);
        assert!(matches!(
            filter.check(
                "https://cdn.ads.example/banner.png",
                ResourceType::Image,
                Some("https://news.example/")
            ),
            RequestDecision::Block { .. }
        ));
        assert!(matches!(
            filter.check(
                "https://cdn.example/analytics-v2.js",
                ResourceType::Script,
                Some("https://news.example/")
            ),
            RequestDecision::Block { .. }
        ));
        assert_eq!(
            filter.check(
                "https://cdn.example/analytics-v2.js",
                ResourceType::Image,
                Some("https://news.example/")
            ),
            RequestDecision::Allow
        );
    }

    #[test]
    fn exception_rules_and_site_allowlist_override_blocking() {
        let mut filter = RequestFilter::default();
        filter.import_adblock_rules("||ads.example^\n@@||safe.ads.example^\n*tracker*");

        assert_eq!(
            filter.check(
                "https://safe.ads.example/pixel",
                ResourceType::Image,
                Some("https://news.example/")
            ),
            RequestDecision::Allow
        );

        filter.allow_site("news.example");
        assert_eq!(
            filter.check(
                "https://ads.example/tracker",
                ResourceType::Image,
                Some("https://sub.news.example/article")
            ),
            RequestDecision::Allow
        );
    }

    #[test]
    fn exposes_deterministic_stats() {
        let mut filter = RequestFilter::default();
        filter.import_adblock_rules("||ads.example^");
        filter.check(
            "https://ads.example/a",
            ResourceType::Image,
            Some("https://site.example"),
        );
        filter.check(
            "https://cdn.example/a",
            ResourceType::Image,
            Some("https://site.example"),
        );
        assert_eq!(
            filter.stats(),
            FilterStats {
                checked: 2,
                allowed: 1,
                blocked: 1,
            }
        );
        filter.reset_stats();
        assert_eq!(filter.stats(), FilterStats::default());
    }
}
