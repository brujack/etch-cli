use std::collections::HashMap;

use anyhow::Result;
use hickory_resolver::Resolver;
use hickory_resolver::config::ResolverConfig;
use hickory_resolver::net::runtime::TokioRuntimeProvider;
use hickory_resolver::proto::rr::{RData, Record};
use reqwest::Url;
use tokio::runtime::Runtime;

pub fn txt_record_values(url: &Url, contexts: &mut HashMap<String, String>) -> Result<()> {
    let resolver =
        Resolver::builder_with_config(resolver_config(), TokioRuntimeProvider::default())
            .build()?;

    let host = url
        .host_str()
        .ok_or_else(|| anyhow::anyhow!("Failed to parse host"))?;

    let rt = Runtime::new()?;
    let lookup = rt.block_on(resolver.txt_lookup(host))?;

    insert_txt_answers(lookup.answers(), contexts);

    Ok(())
}

/// Resolver configuration for TXT lookups: Google Public DNS over UDP and TCP.
///
/// hickory 0.25 made this the `ResolverConfig::default()`; 0.26 derives an empty
/// default (no name servers), so every lookup would fail with "no connections
/// available". Naming the servers keeps the 0.25 behaviour explicit.
fn resolver_config() -> ResolverConfig {
    ResolverConfig::udp_and_tcp(&hickory_resolver::config::GOOGLE)
}

/// Collect `key=value` pairs from the TXT records among `answers`.
///
/// hickory 0.26 returns every answer record (including CNAME chain entries)
/// where 0.25 yielded only TXT data, so non-TXT records must be skipped here.
fn insert_txt_answers(answers: &[Record], contexts: &mut HashMap<String, String>) {
    for record in answers {
        let RData::TXT(txt) = &record.data else {
            continue;
        };
        if let Some((key, value)) = txt.to_string().split_once('=') {
            contexts.insert(key.to_string(), value.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_without_host_is_an_error_and_inserts_nothing() {
        let url = Url::parse("dns+txt:nohost").unwrap();
        let mut contexts = HashMap::new();
        let err = txt_record_values(&url, &mut contexts).unwrap_err();
        assert_eq!(err.to_string(), "Failed to parse host");
        assert!(contexts.is_empty());
    }
    use hickory_resolver::proto::rr::Name;
    use hickory_resolver::proto::rr::rdata::{CNAME, TXT};
    use std::str::FromStr;

    fn record(rdata: RData) -> Record {
        Record::from_rdata(Name::from_str("example.com.").unwrap(), 60, rdata)
    }

    #[test]
    fn resolver_config_uses_google_name_servers() {
        let config = resolver_config();
        let ips: Vec<_> = config.name_servers().iter().map(|ns| ns.ip).collect();
        assert!(!ips.is_empty(), "resolver config has no name servers");
        let google: Vec<_> = hickory_resolver::config::GOOGLE.ips.to_vec();
        assert_eq!(ips, google);
    }

    #[test]
    fn txt_value_keeps_later_equals_signs() {
        let answers = vec![record(RData::TXT(TXT::new(vec!["k=v=w".to_string()])))];
        let mut contexts = HashMap::new();
        insert_txt_answers(&answers, &mut contexts);
        assert_eq!(contexts.get("k").map(String::as_str), Some("v=w"));
        assert_eq!(contexts.len(), 1);
    }

    #[test]
    fn every_txt_answer_is_inserted() {
        let answers = vec![
            record(RData::TXT(TXT::new(vec!["a=1".to_string()]))),
            record(RData::TXT(TXT::new(vec!["b=2".to_string()]))),
        ];
        let mut contexts = HashMap::new();
        insert_txt_answers(&answers, &mut contexts);
        assert_eq!(contexts.get("a").map(String::as_str), Some("1"));
        assert_eq!(contexts.get("b").map(String::as_str), Some("2"));
    }

    #[test]
    fn txt_after_non_txt_answer_is_still_inserted() {
        let answers = vec![
            record(RData::CNAME(CNAME(
                Name::from_labels([b"a=b".as_slice(), b"example".as_slice()]).unwrap(),
            ))),
            record(RData::TXT(TXT::new(vec!["k=v".to_string()]))),
        ];
        let mut contexts = HashMap::new();
        insert_txt_answers(&answers, &mut contexts);
        assert_eq!(contexts.len(), 1);
        assert_eq!(contexts.get("k").map(String::as_str), Some("v"));
    }

    #[test]
    fn txt_answers_become_key_value_pairs() {
        let answers = vec![record(RData::TXT(TXT::new(vec!["key=value".to_string()])))];
        let mut contexts = HashMap::new();
        insert_txt_answers(&answers, &mut contexts);
        assert_eq!(contexts.get("key").map(String::as_str), Some("value"));
    }

    #[test]
    fn non_txt_answers_are_skipped() {
        let answers = vec![record(RData::CNAME(CNAME(
            Name::from_str("target.example.com.").unwrap(),
        )))];
        let mut contexts = HashMap::new();
        insert_txt_answers(&answers, &mut contexts);
        assert!(contexts.is_empty());
    }

    #[test]
    fn txt_without_equals_is_ignored() {
        let answers = vec![record(RData::TXT(TXT::new(vec!["novalue".to_string()])))];
        let mut contexts = HashMap::new();
        insert_txt_answers(&answers, &mut contexts);
        assert!(contexts.is_empty());
    }
}
