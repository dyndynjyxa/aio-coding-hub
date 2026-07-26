# Homebrew Cask Release Notes

This project generates the Homebrew Cask from the release support matrix, then optionally syncs it to a separate tap repository during the release workflow.

## One-time setup

Create the tap repository expected by the README command:

```bash
gh repo create dyndynjyxa/homebrew-aio-coding-hub --public
```

Then configure this repository:

- Secret `HOMEBREW_TAP_TOKEN`: a token that can push to the tap repository.
- Optional variable `HOMEBREW_TAP_REPOSITORY`: defaults to `dyndynjyxa/homebrew-aio-coding-hub`.

The tap repository should contain the generated file at:

```text
Casks/aio-coding-hub.rb
```

## Manual generation

Use the latest release asset digests:

```bash
node scripts/support-matrix.mjs homebrew-cask \
  --tag aio-coding-hub-v0.60.4 \
  --repo dyndynjyxa/aio-coding-hub \
  --macos-arm-sha256 6b126f39ec625e97d182301fafcbfff81ce6f332e297880aef2b0eab0a3c0c4a \
  --macos-intel-sha256 18f376bc6266e8cef4fb3978240ba0247c56b703370f6a95269443c2adbbbcc6 \
  --output Casks/aio-coding-hub.rb
```

Validate before pushing to the tap:

```bash
brew style --cask Casks/aio-coding-hub.rb
```

## Release behavior

On a successful release, `.github/workflows/release.yml` reads the two macOS zip asset digests from GitHub, generates `Casks/aio-coding-hub.rb`, and pushes it to the tap when `HOMEBREW_TAP_TOKEN` is configured. If the token is missing, the release still succeeds and prints a skip message.
