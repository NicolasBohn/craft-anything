use craft_anything_formats::sourcemap::{Limits, SourceMapError, parse};
use serde_json::json;

fn decode(value: serde_json::Value, max_entries: usize) -> Result<craft_anything_formats::sourcemap::SourceMap, SourceMapError> {
    parse(&serde_json::to_vec(&value).unwrap(), Limits { max_entries, ..Limits::default() })
}

#[test]
fn entries_up_to_the_limit_are_kept() {
    let value = json!({"version":3,"sources":["a.js","b.js"],"sourcesContent":["a();",null],"names":["x"],"mappings":"AAAAA,CCAA"});
    let map = decode(value, 5).unwrap();
    assert_eq!(map.sources, vec![Some("a.js".to_string()), Some("b.js".to_string())]);
    assert_eq!(map.sources_content, vec![Some("a();".to_string()), None]);
    assert_eq!(map.names, vec!["x".to_string()]);
}

#[test]
fn sources_content_and_names_share_one_budget() {
    let value = json!({"version":3,"sources":["a.js","b.js"],"sourcesContent":["a();",null],"names":["x"],"mappings":""});
    assert_eq!(decode(value.clone(), 4), Err(SourceMapError::TooManyEntries(4)));
    let only_sources = json!({"version":3,"sources":["a","b","c"],"mappings":""});
    assert_eq!(decode(only_sources, 2), Err(SourceMapError::TooManyEntries(2)));
    let only_names = json!({"version":3,"sources":[],"names":["a","b","c"],"mappings":""});
    assert_eq!(decode(only_names, 2), Err(SourceMapError::TooManyEntries(2)));
    // Entries that are not strings still take a slot each.
    let not_strings = json!({"version":3,"sources":[null, 1, {"a":[1,2,3]}],"mappings":""});
    assert_eq!(decode(not_strings, 2), Err(SourceMapError::TooManyEntries(2)));
    let ignore_list = json!({"version":3,"sources":["a"],"ignoreList":[0,0],"mappings":""});
    assert_eq!(decode(ignore_list, 2), Err(SourceMapError::TooManyEntries(2)));
}

#[test]
fn sections_share_the_budget_with_their_index() {
    let section = |name: &str| json!({"offset":{"line":0,"column":0},"map":{"version":3,"sources":[name],"mappings":"AAAA"}});
    let mut second = section("b.js");
    second["offset"]["line"] = json!(1);
    let value = json!({"version":3,"sections":[section("a.js"), second]});
    assert_eq!(decode(value.clone(), 2).unwrap().sections.len(), 2);
    assert_eq!(decode(value, 1), Err(SourceMapError::TooManyEntries(1)));
}

#[test]
fn reading_stops_at_the_limit() {
    // Everything after the entry that goes over the limit is never read: the broken tail would
    // otherwise be reported as bad JSON.
    let json = br#"{"version":3,"sources":["a","b","c","d" this is not JSON"#;
    let limits = Limits { max_entries: 2, ..Limits::default() };
    assert_eq!(parse(json, limits), Err(SourceMapError::TooManyEntries(2)));
    assert!(matches!(parse(json, Limits::default()), Err(SourceMapError::BadJson(_))));
}

#[test]
fn unknown_fields_are_skipped_without_counting() {
    let value = json!({"version":3,"x_vendor":[1,2,3,4,5,6,7,8],"sources":["a.js"],"mappings":"AAAA"});
    assert_eq!(decode(value, 1).unwrap().sources.len(), 1);
}

#[test]
fn a_hostile_map_of_empty_names_is_refused_by_default() {
    // About 21 million empty strings fit in the 64 MiB a map may have.
    let names = vec!["\"\""; 2_000_001].join(",");
    let json = format!("{{\"version\":3,\"mappings\":\"\",\"names\":[{names}]}}").into_bytes();
    let limit = Limits::default().max_entries;
    assert!(limit < 2_000_001);
    assert_eq!(parse(&json, Limits::default()), Err(SourceMapError::TooManyEntries(limit)));
    // The same map is well formed: with room for every name it reads.
    let roomy = Limits { max_entries: 2_000_001, ..Limits::default() };
    assert_eq!(parse(&json, roomy).unwrap().names.len(), 2_000_001);
}
