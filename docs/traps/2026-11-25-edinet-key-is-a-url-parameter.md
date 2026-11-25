# The EDINET key is a URL parameter, so a URL in a text has the key

*Trap:* The EDINET API v2 takes the key as the query parameter `Subscription-Key`. The documentation lists no header. Each request URL has the key. A client error text, a log line or a saved "request" field that has the URL puts the key in a file or in a chat. The cloud proxy of a session adds a secret to a header, not to a URL, so a cloud session cannot make the request.

*What to do:* Use `mapcheck::edinet` for each EDINET request: it takes the key from `EDINET_API_KEY` (or `.env`), scrubs the key and the parameter value from each error text, and writes `Subscription-Key=<not stored>` in the cache record. Do not print a request URL of EDINET. Run the first real read on a laptop with the key. If you add a new function with a URL, scrub its errors the same way.

*Evidence:* `rust/mapcheck/src/edinet.rs` (`scrub`, `Session::call`); the tests `no_error_text_has_the_key` and `the_cache_has_the_zip_and_a_record_without_the_key_and_the_facts_file_is_made`. Terms and format: EDINET API specification v2 (https://disclosure2dl.edinet-fsa.go.jp/guide/static/disclosure/download/ESE140206.pdf), 3-1-1 and 3-3: some errors (401, 429) come as HTTP 200 with a JSON body, so the code reads the body too.
