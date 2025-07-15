# Branch Protection Configuration

This document describes the recommended branch protection settings for the gitk-rs repository.

## Main Branch Protection

Configure the following settings for the `main` branch:

### Required Status Checks

**Require status checks to pass before merging:**
- ✅ Require branches to be up to date before merging

**Status checks that are required:**
- `check` - Code formatting, linting, and documentation
- `security` - Security audit and dependency checks  
- `test (ubuntu-latest, stable)` - Core platform tests
- `test (windows-latest, stable)` - Windows compatibility
- `test (macos-latest, stable)` - macOS compatibility
- `coverage` - Code coverage requirements
- `msrv` - Minimum Supported Rust Version check

### Merge Requirements

**Restrict pushes that create files:**
- ✅ Require a pull request before merging
- ✅ Require approvals: **2**
- ✅ Dismiss stale reviews when new commits are pushed
- ✅ Require review from code owners
- ✅ Restrict pushes that create files
- ✅ Require conversation resolution before merging

### Administrative Settings

- ✅ Restrict pushes to matching branches
- ✅ Allow force pushes: **Admins only**
- ✅ Allow deletions: **No one**

## Develop Branch Protection

Configure the following settings for the `develop` branch:

### Required Status Checks

**Status checks that are required:**
- `check` - Code formatting, linting, and documentation
- `test (ubuntu-latest, stable)` - Core platform tests
- `coverage` - Code coverage requirements

### Merge Requirements

- ✅ Require a pull request before merging
- ✅ Require approvals: **1**
- ✅ Dismiss stale reviews when new commits are pushed
- ✅ Require conversation resolution before merging

## Quality Gates

### Code Coverage Requirements

```yaml
# codecov.yml
coverage:
  status:
    project:
      default:
        target: 80%
        threshold: 1%
        if_ci_failed: error
    patch:
      default:
        target: 90%
        threshold: 1%
        if_ci_failed: error

comment:
  layout: "reach,diff,flags,tree"
  behavior: default
  require_changes: false
```

### Performance Requirements

- Benchmarks must not regress by more than 10%
- Build time must remain under 5 minutes
- Binary size must not increase by more than 5% without justification

### Security Requirements

- All security checks must pass
- No high or critical vulnerabilities allowed
- License compliance checks must pass
- Dependency audit must be clean

## GitHub CLI Commands

Set up branch protection using GitHub CLI:

```bash
# Main branch protection
gh api repos/:owner/:repo/branches/main/protection \
  --method PUT \
  --field required_status_checks='{"strict":true,"contexts":["check","security","test (ubuntu-latest, stable)","test (windows-latest, stable)","test (macos-latest, stable)","coverage","msrv"]}' \
  --field enforce_admins=true \
  --field required_pull_request_reviews='{"required_approving_review_count":2,"dismiss_stale_reviews":true,"require_code_owner_reviews":true,"require_last_push_approval":true}' \
  --field restrictions='{"users":[],"teams":[],"apps":[]}' \
  --field allow_force_pushes=false \
  --field allow_deletions=false

# Develop branch protection
gh api repos/:owner/:repo/branches/develop/protection \
  --method PUT \
  --field required_status_checks='{"strict":true,"contexts":["check","test (ubuntu-latest, stable)","coverage"]}' \
  --field enforce_admins=false \
  --field required_pull_request_reviews='{"required_approving_review_count":1,"dismiss_stale_reviews":true,"require_code_owner_reviews":false}' \
  --field restrictions=null \
  --field allow_force_pushes=false \
  --field allow_deletions=false
```

## Repository Settings

### General Settings

```yaml
Repository Settings:
  - Default branch: main
  - Merge button: Allow merge commits
  - Squash merging: Allow squash merging
  - Rebase merging: Allow rebase merging
  - Auto-delete head branches: Enabled
  - Automatically delete head branches: Enabled
```

### Security Settings

```yaml
Security:
  - Dependency graph: Enabled
  - Dependabot alerts: Enabled
  - Dependabot security updates: Enabled
  - Dependabot version updates: Enabled
  - Code scanning alerts: Enabled
  - Secret scanning: Enabled
  - Secret scanning push protection: Enabled
```

### Actions Settings

```yaml
Actions:
  - Actions permissions: Allow all actions and reusable workflows
  - Fork pull request workflows: Require approval for first-time contributors
  - Artifact and log retention: 90 days
```

## Rulesets (Alternative Modern Approach)

For repositories using the new GitHub Rulesets feature:

```json
{
  "name": "Main Branch Protection",
  "target": "branch",
  "enforcement": "active",
  "conditions": {
    "ref_name": {
      "include": ["refs/heads/main"],
      "exclude": []
    }
  },
  "rules": [
    {
      "type": "deletion"
    },
    {
      "type": "non_fast_forward"
    },
    {
      "type": "required_status_checks",
      "parameters": {
        "strict_required_status_checks_policy": true,
        "required_status_checks": [
          {
            "context": "check"
          },
          {
            "context": "security"
          },
          {
            "context": "test (ubuntu-latest, stable)"
          },
          {
            "context": "test (windows-latest, stable)"
          },
          {
            "context": "test (macos-latest, stable)"
          },
          {
            "context": "coverage"
          },
          {
            "context": "msrv"
          }
        ]
      }
    },
    {
      "type": "pull_request",
      "parameters": {
        "required_approving_review_count": 2,
        "dismiss_stale_reviews_on_push": true,
        "require_code_owner_review": true,
        "require_last_push_approval": true,
        "required_review_thread_resolution": true
      }
    }
  ]
}
```

## Monitoring and Alerts

### GitHub Notifications

Set up notifications for:
- Failed status checks
- Security alerts
- Dependency updates
- Code scanning alerts

### Slack/Discord Integration

Configure webhooks for:
- Pull request status changes
- Security alert notifications
- Release notifications
- CI/CD pipeline failures

## Manual Override Process

For emergency situations:

1. **Incident Declaration**: Document the emergency
2. **Admin Override**: Repository admin can temporarily disable protection
3. **Emergency Fix**: Apply critical fix with expedited review
4. **Protection Restoration**: Re-enable protection rules
5. **Post-Incident Review**: Review and improve processes

## Quality Gate Automation

### Pre-merge Checklist

Automated checks before merge:
- [ ] All tests pass on all platforms
- [ ] Code coverage meets requirements
- [ ] Security audit passes
- [ ] Documentation is updated
- [ ] Conventional commit format
- [ ] No merge conflicts
- [ ] Branch is up to date

### Post-merge Actions

Automated actions after merge:
- [ ] Deploy to staging environment
- [ ] Run extended test suite
- [ ] Update documentation site
- [ ] Notify relevant teams
- [ ] Update project boards

This configuration ensures high code quality while maintaining development velocity.