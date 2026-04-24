# Android Signing Configuration

This document describes how to set up signing for the CorePolicyManager Android app.

## Local Signing Setup

1. **Generate a keystore** (if you don't have one):
   ```bash
   keytool -genkeypair -v -keystore release.jks -keyalg RSA -keysize 2048 -validity 10000 -alias release
   ```

2. **Place the keystore** in the `app/keystore/` directory (gitignored).

3. **Create a `keystore.properties` file** in the repository root:
   ```properties
   STORE_FILE=app/keystore/release.jks
   STORE_PASSWORD=your_keystore_password
   KEY_ALIAS=release
   KEY_PASSWORD=your_key_password
   ```
   *Note: This file is gitignored and should never be committed.*

## GitHub Actions (CI) Setup

For automated release builds, the signing secrets must be added to a GitHub Environment.

1. **Target Environment**: `production` (or as configured in `build-release.yml`)

2. **Base64 encode your keystore**:
   ```bash
   base64 -w0 app/keystore/release.jks > release.b64
   ```

3. **Add the following Secrets** to the environment:
   - `STORE_FILE_BASE64`: Content of the `release.b64` file.
   - `STORE_PASSWORD`: The password for your keystore.
   - `KEY_ALIAS`: The alias for your key.
   - `KEY_PASSWORD`: The password for your specific key.

## Security Rules

- **Never** commit `.jks`, `.keystore`, or `.properties` files to the repository.
- **Never** print signing passwords in console output or logs.
- The build will fallback to **debug signing** if release secrets are missing.
