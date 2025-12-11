use crate::{ExchangeRate, ExchangeRates};
use regex::Regex;

/// Parse exchange rates from the given string.
///
/// String must be of the [EUROFXREF format](http://www.ecb.int/vocabulary/2002-08-01/eurofxref)
pub fn parse_exchange_rates(input: &str) -> ExchangeRates {
    let timestamp_re = Regex::new(r#"<Cube time='(\d+-\d+-\d+)'>"#).unwrap();
    let (_, [timestamp]) = timestamp_re
        .captures(input)
        .expect("Timestamp must be present in input")
        .extract();

    let sender_re = Regex::new(r#"<gesmes:name>(.+)</gesmes:name>"#).unwrap();
    let (_, [source]) = sender_re
        .captures(input)
        .expect("Sender must be present in input")
        .extract();

    let rates_re = Regex::new(r#"<Cube currency='(\w+)' rate='(\d+)\.(\d{0,5})'/>"#).unwrap();
    let exchange_rates = rates_re
        .captures_iter(input)
        .map(|capture| capture.extract())
        .map(|(_, [currency, rate_whole, rate_decimals])| {
            let rate_whole: u64 = rate_whole
                .parse()
                .expect("Whole part of rate should be a number");
            let missing_decimals: u32 = 5 - rate_decimals.len() as u32;
            let rate_decimals: u64 = rate_decimals
                .parse()
                .expect("Decimal part of rate should be a number");
            ExchangeRate {
                currency: currency.to_owned(),
                rate: rate_whole * 100_000u64 + rate_decimals * 10u64.pow(missing_decimals),
            }
        })
        .collect();

    ExchangeRates {
        timestamp: timestamp.to_string(),
        source: source.to_string(),
        exchange_rates,
    }
}

#[cfg(test)]
mod test {

    use crate::ExchangeRate;

    const TEST_DATA: [&str; 3] = [
        include_str!("../ecb-xml-examples/2025-10-21.xml"),
        include_str!("../ecb-xml-examples/2025-10-24.xml"),
        include_str!("../ecb-xml-examples/2025-11-24.xml"),
    ];

    /// Exchange rates can be parsed from XML strings.
    #[test]
    fn parse_exchange_rates() {
        let result = super::parse_exchange_rates(TEST_DATA[0]);

        assert_eq!(&result.source, "European Central Bank");
        assert_eq!(&result.timestamp, "2025-10-21");

        let exchange_rates = result.exchange_rates;
        assert_eq!(
            exchange_rates[0],
            ExchangeRate {
                currency: "USD".to_owned(),
                rate: 116070
            }
        );
        assert_eq!(
            exchange_rates[1],
            ExchangeRate {
                currency: "JPY".to_owned(),
                rate: 17645000
            }
        );
        assert_eq!(
            exchange_rates[4],
            ExchangeRate {
                currency: "DKK".to_owned(),
                rate: 746910
            }
        );
    }

    /// Different files can be parsed.
    #[test]
    fn parse_exchange_rates_param() {
        for xml_data in TEST_DATA {
            let result = super::parse_exchange_rates(xml_data);
            assert_eq!(&result.source, "European Central Bank");
        }
    }

    /// Parsing invalid XML strings will panic
    #[test]
    #[should_panic]
    fn parse_empty() {
        super::parse_exchange_rates("");
    }
}
