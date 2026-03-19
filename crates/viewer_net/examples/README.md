# Manual LLSD Login Attempt

This example performs a controlled login attempt through `viewer_net` using:
- `LoginWireFormat::Llsd`
- `Connection::login_with_trace(...)`
- `SecondLifeAdapter`

It is manual-only and requires environment variables. It does not store credentials.

## Required env vars
- `VIEWER_LOGIN_ENDPOINT`
- `VIEWER_LOGIN_USERNAME`
- `VIEWER_LOGIN_PASSWORD`

## Optional env vars
- `VIEWER_LOGIN_START` (`last` | `home` | URI-like string, default: `last`)
- `VIEWER_LOGIN_AGREE_TOS` (`true/false`, default: `false`)
- `VIEWER_LOGIN_READ_CRITICAL` (`true/false`, default: `true`)
- `VIEWER_LOGIN_MFA_TOKEN`
- `VIEWER_LOGIN_TIMEOUT_SECS` (default: `15`)

## Run
```powershell
$env:VIEWER_LOGIN_ENDPOINT="https://your-grid-login-endpoint"
$env:VIEWER_LOGIN_USERNAME="your.username"
$env:VIEWER_LOGIN_PASSWORD="your-password"
cargo run -p viewer_net --example llsd_login_attempt
```

## Output
- High-level login outcome (`success`, `failed`, `requires_tos`, etc.)
- Sanitized `LoginTrace`
- Interpreted typed result or transport/protocol error details
