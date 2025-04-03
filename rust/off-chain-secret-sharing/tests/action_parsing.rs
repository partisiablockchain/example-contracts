//! Test parsing of http requests

use pbc_contract_common::off_chain::HttpRequestData;

use off_chain_secret_sharing::{parse_action, HttpAction};

/// Parse valid HTTP actions.
#[test]
fn parse_valid_http_actions() {
    let l1 = parse_action(&http_request("GET", "/shares/123"));
    let s1 = parse_action(&http_request("PUT", "/shares/123"));

    assert_eq!(l1, Some(HttpAction::Load { sharing_id: 123 }));
    assert_eq!(s1, Some(HttpAction::Store { sharing_id: 123 }));
}

/// Cannot parse for invalid method
#[test]
fn cannot_parse_for_invalid_method() {
    let post1 = parse_action(&http_request("POST", "/shares/123"));
    assert_eq!(post1, None);
}

/// Cannot parse unknown paths
#[test]
fn cannot_parse_unknown_paths() {
    let paths = vec![
        "",
        "/",
        "/shares",
        "/shares/",
        "/shares/123/",
        "/shares/34124/1321/",
        "/smares/123",
        "/notshares/123",
        "smerp",
        "smerp/",
        "smerp/shares",
        "smerp/shares/",
        "smerp/shares/123",
        "smerp/shares/123/",
        "smerp/shares/34124/1321/",
        "smerp/smares/123",
        "smerp/notshares/123",
    ];
    for path in paths {
        assert_eq!(parse_action(&http_request("GET", path)), None, "{path}");
        assert_eq!(parse_action(&http_request("PUT", path)), None, "{path}");
    }
}

fn http_request(method: &str, uri: &str) -> HttpRequestData {
    HttpRequestData {
        method: method.to_string(),
        uri: uri.to_string(),
        body: vec![],
        headers: vec![],
    }
}
