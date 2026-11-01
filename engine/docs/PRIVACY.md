# Privacy rules

`cutroom privacy work/` OCRs the screen recordings and finds secrets: API keys and tokens
(OpenAI/Anthropic, GitHub, Google, Slack, AWS, Stripe, JWTs, bearer tokens, keys in URLs and after
`key=`/`secret:`/`password`), emails, phone numbers and card numbers. Each finding becomes a blur
beat in `beats/privacy.yaml`, anchored to source time. `check` reports any finding that would be
visible unblurred as an error and `render` refuses to run until it is fixed.

## Project rules: `privacy-rules.yaml`

Put a `privacy-rules.yaml` in the work folder (see `docs/privacy-rules.example.yaml`) to add what
only this project knows is private:

```yaml
patterns:
  - {name: community-price, regex: '\$\s?\d+(?:\.\d{2})?\s*/\s*(?:mo|month)'}
allow: ["@aianswer.us"]
skip: [phone]
```

- `patterns`: Rust regexes. They run on each OCR'd line with single spaces between words and on
  space-free joins, so `$97 / month` split into three OCR words still matches. The secret is the
  capture group named `v`, else group 1, else the whole match. `name` is the finding's kind.
- `allow`: values that may stay visible (exact, `@domain`, or `/regex/`).
- `skip`: built-in kinds to leave out.
- Every regex is compiled before the scan starts, so a typo fails immediately.

The same from the command line, repeatable and added to the file's rules:

```
cutroom privacy work/ --pattern 'community-price=\$\s?\d+\s*/\s*mo' --allow me@example.com
cutroom privacy work/ --rules ~/rules/course.yaml
```

Measured on a 15-minute section of a real course with two custom patterns and one from the
command line: 30 findings (11 + 18 custom, 1 email) in 30 s, all turned into blur beats;
removing the blur beats makes `check` list every one and `render` refuse.
