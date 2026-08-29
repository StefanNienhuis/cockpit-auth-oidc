use openidconnect::core::{CoreClient, CoreProviderMetadata};
use openidconnect::{AccessTokenHash, AuthorizationCode, ClientId, ClientSecret, IssuerUrl, Nonce, NonceVerifier, OAuth2TokenResponse, RedirectUrl, TokenResponse};
use openidconnect::reqwest;

struct NoopNonceVerifier;

impl NonceVerifier for NoopNonceVerifier {
    fn verify(self, _: Option<&Nonce>) -> Result<(), String> {
        Ok(())
    }
}

/// Exchange an OIDC token.
/// Returns the username if it is valid or a string error if not.
pub(crate) fn exchange(code: &str, client_id: &str, client_secret: &str, issuer_url: &str, redirect_url: &str) -> Result<String, String> {
    let http_client = reqwest::blocking::ClientBuilder::new()
        // Following redirects opens the client up to SSRF vulnerabilities.
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|err| format!("Failed to build HTTP client: {:?}", err))?;

    let provider_metadata = CoreProviderMetadata::discover(
        &IssuerUrl::new(issuer_url.to_string())
            .map_err(|err| format!("Failed to build issuer URL: {:?}", err))?,
        &http_client
    );

    let client = CoreClient::from_provider_metadata(
        provider_metadata
            .map_err(|err| format!("Failed to build provider metadata: {:?}", err))?,
        ClientId::new(client_id.to_string()),
        Some(ClientSecret::new(client_secret.to_string()))
    ).set_redirect_uri(
        RedirectUrl::new(redirect_url.to_string())
            .map_err(|err| format!("Failed to build redirect URL: {:?}", err))?
    );

    let token_response =
        client
            .exchange_code(AuthorizationCode::new(code.to_string()))
            .map_err(|err| format!("Failed to build get token request: {:?}", err))?
            .request(&http_client)
            .map_err(|err| format!("Failed to get token: {:?}", err))?;

    // Extract the ID token claims after verifying its authenticity and nonce.
    let id_token = token_response
        .id_token()
        .ok_or("Missing ID token".to_string())?;
    let id_token_verifier = client.id_token_verifier();

    let claims = id_token
        .claims(&client.id_token_verifier(), NoopNonceVerifier)
        .map_err(|err| format!("Failed to get claims: {:?}", err))?;

    // Verify the access token hash to ensure that the access token hasn't been substituted for
    // another user's.
    if let Some(expected_access_token_hash) = claims.access_token_hash() {
        let actual_access_token_hash = AccessTokenHash::from_token(
            token_response.access_token(),
            id_token.signing_alg()
                    .map_err(|err| format!("Error while getting signing algorithm: {:?}", err))?,
            id_token.signing_key(&id_token_verifier)
                .map_err(|err| format!("Error while getting signing key: {:?}", err))?
        ).map_err(|err| format!("Error while getting access token hash: {:?}", err))?;

        if actual_access_token_hash != *expected_access_token_hash {
            return Err("Invalid access token hash".to_string());
        }
    }

    match claims.preferred_username() {
        Some(username) => Ok(username.to_string()),
        None => Err("Missing preferred username claim".to_string())
    }
}