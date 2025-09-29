# Security Policy

Contact: security@deepersensor.com
Expires: 2026-12-31T23:59:59.000Z
Preferred-Languages: en
Canonical: https://deepersensor.com/.well-known/security.txt
Encryption: https://deepersensor.com/pgp-key.txt
Acknowledgments: https://deepersensor.com/security/hall-of-fame

## Reporting a Vulnerability

If you discover a security vulnerability in DeeperSensor API, please report it responsibly:

1. **DO NOT** open a public GitHub issue
2. Email security@deepersensor.com with:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if any)

## Response Timeline

- Initial response: Within 48 hours
- Vulnerability assessment: Within 5 business days
- Fix timeline: Depends on severity (critical: 24-48h, high: 1 week, medium: 2 weeks)

## Scope

In scope:
- Authentication bypass
- SQL injection
- Remote code execution
- Privilege escalation
- XSS and CSRF
- Data leakage
- Rate limit bypass
- JWT token vulnerabilities

Out of scope:
- Social engineering
- Physical attacks
- DoS requiring extreme resources
- Issues in third-party dependencies (report to maintainers directly)
- Issues requiring outdated/unsupported software

## Responsible Disclosure

We request that you:
- Give us reasonable time to fix issues before public disclosure
- Make a good faith effort to avoid privacy violations, data destruction, and service interruption
- Do not exploit the vulnerability beyond what is necessary to demonstrate it

## Recognition

We maintain a Hall of Fame for security researchers who responsibly disclose vulnerabilities.
