# fetch-exchange-rate
Example Oracle that maintains a list of exchange rates between the Euro and other currencies.
The oracle makes these exchanges rates available on the blockchain.

Data is sourced from the [European Central Bank](https://www.ecb.europa.eu/stats/policy_and_exchange_rates/euro_reference_exchange_rates/html/index.en.html),
and parsed from their daily XML feed. Update of the exchange rate must be
manually triggered by calling [`refresh_exchange_rates()`].

