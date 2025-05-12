# Security Policy

## Supported Versions
Only the latest version receives security updates.

## Reporting a Vulnerability
- **Do not disclose publicly** until addressed
- Email: nikolai@netlife.no with subject "qrlyzer Security Vulnerability"
- Include: description, reproduction steps, and impact

## Security Scope
**In Scope:**
- Buffer overflows, memory issues
- Denial of service vulnerabilities
- Input validation flaws
- Path traversal issues

**Out of Scope:**
- Content of decoded QR codes (validate in your application)
- Social engineering
- Issues requiring physical access

## Consumer Best Practices
- Keep qrlyzer updated
- Validate QR code content before use
- Implement proper error handling
- Apply resource limits when processing large images
- Sanitize file paths before passing to library

## Distribution Security
All PyPI packages are signed using sigstore for verification.