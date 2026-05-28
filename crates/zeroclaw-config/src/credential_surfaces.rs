//! Explicit classification for credential-shaped config surfaces.
//!
//! The registry is deliberately separate from `#[secret]`: some sensitive-looking
//! paths are environment variable names, file paths, external auth stores, or
//! known follow-ups rather than values that should be encrypted by SecretStore.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CredentialSurfaceClass {
    EncryptedSecret,
    PathOnlyReference,
    PublicValue,
    ExternalAuthStore,
    LegacyEnvPath,
    RequiresFollowUp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CredentialSurfaceInfo {
    path_pattern: &'static str,
    pub(crate) class: CredentialSurfaceClass,
    rationale: &'static str,
}

const CREDENTIAL_SURFACES: &[CredentialSurfaceInfo] = &[
    CredentialSurfaceInfo {
        path_pattern: "providers.models.*.*.api-key",
        class: CredentialSurfaceClass::EncryptedSecret,
        rationale: "model provider API keys are stored through SecretStore",
    },
    CredentialSurfaceInfo {
        path_pattern: "providers.models.*.*.extra-headers",
        class: CredentialSurfaceClass::EncryptedSecret,
        rationale: "custom provider headers can carry authorization material",
    },
    CredentialSurfaceInfo {
        path_pattern: "providers.models.*.*.requires-openai-auth",
        class: CredentialSurfaceClass::ExternalAuthStore,
        rationale: "boolean selector for OPENAI_API_KEY or ~/.codex/auth.json, not credential material",
    },
    CredentialSurfaceInfo {
        path_pattern: "providers.transcription.*.*.api-key",
        class: CredentialSurfaceClass::EncryptedSecret,
        rationale: "transcription provider API keys are stored through SecretStore",
    },
    CredentialSurfaceInfo {
        path_pattern: "providers.tts.*.*.api-key",
        class: CredentialSurfaceClass::EncryptedSecret,
        rationale: "TTS provider API keys are stored through SecretStore",
    },
    CredentialSurfaceInfo {
        path_pattern: "providers.transcription.local-whisper.*.bearer-token",
        class: CredentialSurfaceClass::EncryptedSecret,
        rationale: "local Whisper provider bearer tokens are stored through SecretStore",
    },
    CredentialSurfaceInfo {
        path_pattern: "model-routes.*.api-key",
        class: CredentialSurfaceClass::RequiresFollowUp,
        rationale: "route objects need a focused object-array secret handling slice",
    },
    CredentialSurfaceInfo {
        path_pattern: "embedding-routes.*.api-key",
        class: CredentialSurfaceClass::RequiresFollowUp,
        rationale: "embedding route objects need a focused object-array secret handling slice",
    },
    CredentialSurfaceInfo {
        path_pattern: "mcp.servers.*.env",
        class: CredentialSurfaceClass::EncryptedSecret,
        rationale: "stdio MCP environment values can carry upstream credentials",
    },
    CredentialSurfaceInfo {
        path_pattern: "mcp.servers.*.headers",
        class: CredentialSurfaceClass::EncryptedSecret,
        rationale: "HTTP/SSE MCP headers commonly carry bearer tokens",
    },
    CredentialSurfaceInfo {
        path_pattern: "channels.*.*.access-token",
        class: CredentialSurfaceClass::EncryptedSecret,
        rationale: "channel access tokens are stored through SecretStore",
    },
    CredentialSurfaceInfo {
        path_pattern: "channels.*.*.bot-token",
        class: CredentialSurfaceClass::EncryptedSecret,
        rationale: "channel bot tokens are stored through SecretStore",
    },
    CredentialSurfaceInfo {
        path_pattern: "channels.*.*.recovery-key",
        class: CredentialSurfaceClass::EncryptedSecret,
        rationale: "channel recovery keys are stored through SecretStore",
    },
    CredentialSurfaceInfo {
        path_pattern: "channels.*.*.password",
        class: CredentialSurfaceClass::EncryptedSecret,
        rationale: "channel login passwords are stored through SecretStore",
    },
    CredentialSurfaceInfo {
        path_pattern: "node-transport.shared-secret",
        class: CredentialSurfaceClass::EncryptedSecret,
        rationale: "node transport shared secret authenticates signed requests",
    },
    CredentialSurfaceInfo {
        path_pattern: "storage.qdrant.*.api-key",
        class: CredentialSurfaceClass::EncryptedSecret,
        rationale: "Qdrant API keys are stored through SecretStore",
    },
    CredentialSurfaceInfo {
        path_pattern: "observability.otel-headers",
        class: CredentialSurfaceClass::EncryptedSecret,
        rationale: "OTLP headers may include authorization tokens",
    },
    CredentialSurfaceInfo {
        path_pattern: "reliability.api-keys",
        class: CredentialSurfaceClass::EncryptedSecret,
        rationale: "rate-limit rotation keys are provider credentials",
    },
    CredentialSurfaceInfo {
        path_pattern: "file-upload.headers",
        class: CredentialSurfaceClass::EncryptedSecret,
        rationale: "upload headers may authenticate to the configured endpoint",
    },
    CredentialSurfaceInfo {
        path_pattern: "composio.api-key",
        class: CredentialSurfaceClass::EncryptedSecret,
        rationale: "Composio API key is stored through SecretStore",
    },
    CredentialSurfaceInfo {
        path_pattern: "browser.computer-use.api-key",
        class: CredentialSurfaceClass::EncryptedSecret,
        rationale: "computer-use sidecar bearer token is stored through SecretStore",
    },
    CredentialSurfaceInfo {
        path_pattern: "web-search.brave-api-key",
        class: CredentialSurfaceClass::EncryptedSecret,
        rationale: "Brave search API key is stored through SecretStore",
    },
    CredentialSurfaceInfo {
        path_pattern: "web-search.tavily-api-key",
        class: CredentialSurfaceClass::EncryptedSecret,
        rationale: "Tavily search API key is stored through SecretStore",
    },
    CredentialSurfaceInfo {
        path_pattern: "transcription.api-key",
        class: CredentialSurfaceClass::EncryptedSecret,
        rationale: "legacy transcription API key is stored through SecretStore",
    },
    CredentialSurfaceInfo {
        path_pattern: "transcription.*.api-key",
        class: CredentialSurfaceClass::EncryptedSecret,
        rationale: "typed transcription API keys are stored through SecretStore",
    },
    CredentialSurfaceInfo {
        path_pattern: "transcription.local-whisper.bearer-token",
        class: CredentialSurfaceClass::EncryptedSecret,
        rationale: "local Whisper bearer token is stored through SecretStore",
    },
    CredentialSurfaceInfo {
        path_pattern: "notion.api-key",
        class: CredentialSurfaceClass::EncryptedSecret,
        rationale: "Notion integration API key is stored through SecretStore",
    },
    CredentialSurfaceInfo {
        path_pattern: "jira.api-token",
        class: CredentialSurfaceClass::EncryptedSecret,
        rationale: "Jira API token is stored through SecretStore",
    },
    CredentialSurfaceInfo {
        path_pattern: "security.nevis.client-secret",
        class: CredentialSurfaceClass::EncryptedSecret,
        rationale: "OAuth client secret is stored through SecretStore",
    },
    CredentialSurfaceInfo {
        path_pattern: "security.oauth.*.client-secret",
        class: CredentialSurfaceClass::EncryptedSecret,
        rationale: "OAuth client secrets are stored through SecretStore",
    },
    CredentialSurfaceInfo {
        path_pattern: "microsoft365.client-secret",
        class: CredentialSurfaceClass::EncryptedSecret,
        rationale: "Microsoft 365 client secret is stored through SecretStore",
    },
    CredentialSurfaceInfo {
        path_pattern: "nodes.auth-token",
        class: CredentialSurfaceClass::EncryptedSecret,
        rationale: "dynamic node discovery bearer token is stored through SecretStore",
    },
    CredentialSurfaceInfo {
        path_pattern: "gateway.paired-tokens",
        class: CredentialSurfaceClass::EncryptedSecret,
        rationale: "gateway pairing tokens are stored through SecretStore",
    },
    CredentialSurfaceInfo {
        path_pattern: "tunnel.cloudflare.token",
        class: CredentialSurfaceClass::EncryptedSecret,
        rationale: "Cloudflare tunnel token is stored through SecretStore",
    },
    CredentialSurfaceInfo {
        path_pattern: "tunnel.ngrok.auth-token",
        class: CredentialSurfaceClass::EncryptedSecret,
        rationale: "ngrok auth token is stored through SecretStore",
    },
    CredentialSurfaceInfo {
        path_pattern: "tunnel.pinggy.token",
        class: CredentialSurfaceClass::EncryptedSecret,
        rationale: "Pinggy tunnel token is stored through SecretStore",
    },
    CredentialSurfaceInfo {
        path_pattern: "tunnel.openvpn.auth-file",
        class: CredentialSurfaceClass::PathOnlyReference,
        rationale: "path to OpenVPN auth material, not the credential value",
    },
    CredentialSurfaceInfo {
        path_pattern: "google-workspace.credentials-path",
        class: CredentialSurfaceClass::PathOnlyReference,
        rationale: "path to external credential material, not the credential value",
    },
    CredentialSurfaceInfo {
        path_pattern: "claude-code.env-passthrough",
        class: CredentialSurfaceClass::LegacyEnvPath,
        rationale: "names of environment variables intentionally passed to the CLI",
    },
    CredentialSurfaceInfo {
        path_pattern: "codex-cli.env-passthrough",
        class: CredentialSurfaceClass::LegacyEnvPath,
        rationale: "names of environment variables intentionally passed to the CLI",
    },
    CredentialSurfaceInfo {
        path_pattern: "gemini-cli.env-passthrough",
        class: CredentialSurfaceClass::LegacyEnvPath,
        rationale: "names of environment variables intentionally passed to the CLI",
    },
    CredentialSurfaceInfo {
        path_pattern: "opencode-cli.env-passthrough",
        class: CredentialSurfaceClass::LegacyEnvPath,
        rationale: "names of environment variables intentionally passed to the CLI",
    },
    CredentialSurfaceInfo {
        path_pattern: "risk-profiles.*.shell-env-passthrough",
        class: CredentialSurfaceClass::LegacyEnvPath,
        rationale: "names of environment variables intentionally allowed through shell policy",
    },
    CredentialSurfaceInfo {
        path_pattern: "web-fetch.api-key-env",
        class: CredentialSurfaceClass::LegacyEnvPath,
        rationale: "environment variable name used for Firecrawl credentials",
    },
    CredentialSurfaceInfo {
        path_pattern: "web-fetch.firecrawl.api-key-env",
        class: CredentialSurfaceClass::LegacyEnvPath,
        rationale: "environment variable name used for Firecrawl credentials",
    },
    CredentialSurfaceInfo {
        path_pattern: "linkedin.image.*.api-key-env",
        class: CredentialSurfaceClass::LegacyEnvPath,
        rationale: "environment variable name used for LinkedIn image provider credentials",
    },
    CredentialSurfaceInfo {
        path_pattern: "linkedin.image.imagen.project-id-env",
        class: CredentialSurfaceClass::LegacyEnvPath,
        rationale: "environment variable name used for Google Imagen project routing",
    },
    CredentialSurfaceInfo {
        path_pattern: "image-gen.*.api-key-env",
        class: CredentialSurfaceClass::LegacyEnvPath,
        rationale: "environment variable name used for image provider credentials",
    },
    CredentialSurfaceInfo {
        path_pattern: "image-gen.api-key-env",
        class: CredentialSurfaceClass::LegacyEnvPath,
        rationale: "environment variable name used for the generic image provider credential",
    },
    CredentialSurfaceInfo {
        path_pattern: "gateway.trust-forwarded-headers",
        class: CredentialSurfaceClass::PublicValue,
        rationale: "boolean proxy-header trust setting, not header credential material",
    },
    CredentialSurfaceInfo {
        path_pattern: "secrets.encrypt",
        class: CredentialSurfaceClass::PublicValue,
        rationale: "boolean SecretStore encryption toggle, not credential material",
    },
];

pub(crate) fn classify(path: &str) -> Option<&'static CredentialSurfaceInfo> {
    CREDENTIAL_SURFACES
        .iter()
        .find(|surface| path_matches(surface.path_pattern, path))
}

pub(crate) fn is_credential_shaped_path(path: &str) -> bool {
    path.split('.').any(|part| {
        let has_term = |needle| part.split('-').any(|term| term == needle);
        part.contains("api-key")
            || part.contains("api-token")
            || part.contains("auth-file")
            || part.contains("auth-header")
            || part.contains("auth-token")
            || part.contains("bearer-token")
            || part.contains("bot-token")
            || part.contains("access-token")
            || part.contains("refresh-token")
            || part.contains("verification-token")
            || part.contains("paired-tokens")
            || part == "token"
            || has_term("credential")
            || has_term("env")
            || has_term("header")
            || has_term("headers")
            || has_term("password")
            || has_term("secret")
    })
}

fn path_matches(pattern: &str, path: &str) -> bool {
    let mut pattern_parts = pattern.split('.');
    let mut path_parts = path.split('.');

    loop {
        match (pattern_parts.next(), path_parts.next()) {
            (None, None) => return true,
            (Some(pattern), Some(part)) if pattern == "*" || pattern == part => {}
            _ => return false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wildcard_patterns_match_single_path_component() {
        assert!(path_matches(
            "providers.models.*.*.api-key",
            "providers.models.anthropic.default.api-key"
        ));
        assert!(!path_matches(
            "providers.models.*.*.api-key",
            "providers.models.anthropic.api-key"
        ));
        assert!(!path_matches(
            "providers.models.*.*.api-key",
            "providers.models.anthropic.default.nested.api-key"
        ));

        assert!(
            CREDENTIAL_SURFACES
                .iter()
                .all(|surface| !surface.rationale.is_empty()),
            "credential-surface classifications should explain their rationale"
        );
    }
}
