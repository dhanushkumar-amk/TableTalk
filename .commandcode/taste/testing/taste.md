# Testing

- Prefers deterministic tests with a fixed clock and a seeded database. Confidence: 0.8
- Does not weaken a test to make it pass; if a race test is flaky, fixes the code instead. Confidence: 0.85
- Claims something works only after running it and showing the output. Confidence: 0.85
- Values concurrency/race tests run over many iterations for a single consistent outcome. Confidence: 0.8
- Keeps existing behavior backward compatible; existing tests must pass unchanged and new parameters are optional. Confidence: 0.8
- Avoids real PII or card numbers in tests; uses clearly synthetic data. Confidence: 0.75
