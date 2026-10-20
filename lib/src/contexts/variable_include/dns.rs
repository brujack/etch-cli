use std::collections::HashMap;

use anyhow::Result;
use hickory_resolver::config::ResolverConfig;
use hickory_resolver::net::runtime::TokioRuntimeProvider;
use hickory_resolver::proto::rr::{RData, Record};
use hickory_resolver::Resolver;
use reqwest::Url;
use tokio::runtime::Runtime;

pub fn txt_record_values(url: &Url, contexts: &mut HashMap<String, String>) -> Result<()> {
    let resolver =
        Resolver::builder_with_config(ResolverConfig::default(), TokioRuntimeProvider::default())
            .build()?;

    let host = url
        .host_str()
        .ok_or_else(|| anyhow::anyhow!("Failed to parse host"))?;

    let rt = Runtime::new()?;
    let lookup = rt.block_on(resolver.txt_lookup(host))?;

    insert_txt_answers(lookup.answers(), contexts);

    Ok(())
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
    use hickory_resolver::proto::rr::rdata::{CNAME, TXT};
    use hickory_resolver::proto::rr::Name;
    use std::str::FromStr;

    fn record(rdata: RData) -> Record {
        Record::from_rdata(Name::from_str("example.com.").unwrap(), 60, rdata)
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
