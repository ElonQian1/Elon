# APK release signing secrets

`ELON_RELEASE_STORE_PASSWORD` and `ELON_RELEASE_KEY_PASSWORD` are APK release
signing secrets. Their plaintext values must not be written to this repository,
documentation, commit history, PR comments, logs, or chat transcripts.

The release scripts and Gradle config read these values from the local machine:

- current process or user environment variables
- `~/.gradle/gradle.properties`
- `android/local.properties`

Safe local PowerShell examples:

```powershell
# Current shell only
$env:ELON_RELEASE_STORE_PASSWORD = "<store password from the maintainer's secret store>"
$env:ELON_RELEASE_KEY_PASSWORD = "<key password from the maintainer's secret store>"

# Persistent user-level environment variables
[Environment]::SetEnvironmentVariable("ELON_RELEASE_STORE_PASSWORD", "<store password from the maintainer's secret store>", "User")
[Environment]::SetEnvironmentVariable("ELON_RELEASE_KEY_PASSWORD", "<key password from the maintainer's secret store>", "User")
```

The expected default keystore path is:

```text
~/.elon/signing/elon-release.jks
```

If either password is missing, `scripts/publish-apk.ps1` stops before publishing
and prints the missing variable names. Do not bypass that check by committing
the secret values to Git.
