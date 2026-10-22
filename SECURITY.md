# Security policy

## Supported versions

R-Term is pre-1.0 software. Security fixes are applied to the latest commit on
`main`; older snapshots and development branches are not supported.

## Reporting a vulnerability

Use GitHub's private **Security → Report a vulnerability** flow for this
repository. If that flow is unavailable, contact the repository owner through
their GitHub profile and request a private channel. Do not publish exploit
details, credentials, host keys, bootstrap URLs, or terminal transcripts in a
public issue.

Include the affected commit, operating system, backend (`OpenSSH` or native
`russh`), reproduction steps, impact, and any proposed mitigation. Maintainers
should acknowledge a report within seven days and coordinate disclosure after
a fix is available.

## Security boundaries

- The Web terminal listens on loopback only. Its URL contains a 60-second,
  single-use bootstrap ticket that is exchanged for a separate session cookie.
- Remote SSH sessions disable OSC 52 clipboard access by default. Enabling
  `--osc52 write` or `--osc52 read-write` trusts terminal output to interact
  with the local clipboard; decoded clipboard writes are limited to 1 MiB.
- Dynamic SOCKS5 forwarding supports loopback listeners only because the
  implemented SOCKS5 method has no client authentication.
- Unknown native SSH host keys are rejected unless the user explicitly selects
  another host-key policy.
- Native RSA private-key authentication is disabled while the upstream Rust RSA
  timing-side-channel advisory has no fix; the system OpenSSH backend remains
  available for legacy RSA identities.
- Command-line secrets are rejected. Passwords and key passphrases belong in
  interactive prompts or an SSH agent.

See [the dependency policy](docs/dependency-policy.md) for audit and supply-chain
controls.
