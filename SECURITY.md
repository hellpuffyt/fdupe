# Security policy

## Supported versions

Only the latest release on the default branch receives security fixes.

## Reporting a vulnerability

Please **do not** open a public issue for a suspected vulnerability.
Report it privately through GitHub's advisory form:

https://github.com/hellpuffyt/fdupe/security/advisories/new

Include the version or commit, a description of the issue, and steps or
input that reproduce it. You should receive an acknowledgement within a
few days; once a fix is available it will be released and credited in
the advisory unless you prefer otherwise.

## Scope

`fdupe` can delete files. Anything that could make it remove, modify or overwrite a file without `--delete` (or a `--manifest` output path the user named), or delete the last remaining copy of some content, is in scope and treated as critical. Note that a manifest is trusted input: `--from-manifest --delete` removes the paths it lists (after the checks described in the README), so only execute manifests you have reviewed.
