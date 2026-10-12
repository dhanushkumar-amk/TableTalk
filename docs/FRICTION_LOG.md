# Friction Log

A running log of unexpected friction, hurdles, bugs, or developer experience issues encountered during development.

## Entry Template

```markdown
### [Task attempted]
- **Date**: YYYY-MM-DD
- **Task attempted**: Brief description of the goal
- **Steps taken**: 
  1. Step one
  2. Step two
- **Expected vs actual**: 
  - *Expected*: What was supposed to happen
  - *Actual*: What actually happened (including error messages)
- **Severity**: Low | Medium | High | Critical
- **Workaround**: How it was resolved or bypassed
- **Suggestion**: How this could be improved or avoided
```

---

## Log Entries

### EXAMPLE - delete me
- **Date**: 2026-10-12
- **Task attempted**: Initializing Axum HTTP health check server with custom environment port binding
- **Steps taken**:
  1. Added Axum dependency with Tokio runtime
  2. Read `PORT` variable from environment with default fallback to 8080
  3. Tested binding to `0.0.0.0:8080`
- **Expected vs actual**:
  - *Expected*: Seamless compile and bind to 0.0.0.0
  - *Actual*: Port was occupied by another local service during initial test
- **Severity**: Low
- **Workaround**: Killed the stale process occupying port 8080 before binding
- **Suggestion**: Provide clear log messages displaying the bound host and port on server startup
