use std::io;

use anyhow::Result;
use reqwest::blocking::{Request, Response};
use reqwest::header::{HeaderValue, AUTHORIZATION, WWW_AUTHENTICATE};
use reqwest::StatusCode;

use crate::cli::AuthType;
use crate::middleware::{Context, Middleware};
use crate::netrc;
use crate::regex;
use crate::utils::clone_request;

#[derive(Debug, PartialEq, Eq)]
pub enum Auth {
    Bearer(String),
    Basic(String, Option<String>),
    Digest(String, String),
}

impl Auth {
    pub fn from_str(auth: &str, auth_type: AuthType, host: &str) -> Result<Auth> {
        match auth_type {
            AuthType::basic => {
                let (username, password) = parse_auth(auth, host)?;
                Ok(Auth::Basic(username, password))
            }
            AuthType::digest => {
                let (username, password) = parse_auth(auth, host)?;
                Ok(Auth::Digest(
                    username,
                    password.unwrap_or_else(|| "".into()),
                ))
            }
            AuthType::bearer => Ok(Auth::Bearer(auth.into())),
        }
    }

    pub fn from_netrc(auth_type: AuthType, entry: netrc::Entry) -> Option<Auth> {
        match auth_type {
            AuthType::basic => Some(Auth::Basic(entry.login?, Some(entry.password))),
            AuthType::bearer => Some(Auth::Bearer(entry.password)),
            AuthType::digest => Some(Auth::Digest(entry.login?, entry.password)),
        }
    }

    // Could be a method on AuthType instead, but it's helpful
    // to put next to `Auth::from_netrc`
    pub fn supports_netrc(auth_type: AuthType) -> bool {
        match auth_type {
            AuthType::basic => true,
            AuthType::bearer => true,
            AuthType::digest => true,
        }
    }
}

pub fn parse_auth(auth: &str, host: &str) -> io::Result<(String, Option<String>)> {
    if let Some(cap) = regex!(r"^([^:]*):$").captures(auth) {
        Ok((cap[1].to_string(), None))
    } else if let Some(cap) = regex!(r"^(.+?):(.+)$").captures(auth) {
        let username = cap[1].to_string();
        let password = cap[2].to_string();
        Ok((username, Some(password)))
    } else {
        let username = auth.to_string();
        let prompt = format!("http: password for {}@{}: ", username, host);
        let password = rpassword::read_password_from_tty(Some(&prompt))?;
        Ok((username, Some(password)))
    }
}

pub struct DigestAuthMiddleware<'a> {
    username: &'a str,
    password: &'a str,
}

impl<'a> DigestAuthMiddleware<'a> {
    pub fn new(username: &'a str, password: &'a str) -> Self {
        DigestAuthMiddleware { username, password }
    }
}

impl<'a> Middleware for DigestAuthMiddleware<'a> {
    fn handle(&mut self, mut ctx: Context, mut request: Request) -> Result<Response> {
        let mut response = self.next(&mut ctx, clone_request(&mut request)?)?;
        match response.headers().get(WWW_AUTHENTICATE) {
            Some(wwwauth) if response.status() == StatusCode::UNAUTHORIZED => {
                let mut context = digest_auth::AuthContext::new(
                    self.username,
                    self.password,
                    request.url().path(),
                );
                if let Some(cnonc) = std::env::var_os("XH_TEST_DIGEST_AUTH_CNONCE") {
                    context.set_custom_cnonce(cnonc.to_string_lossy().to_string());
                }
                let mut prompt = digest_auth::parse(wwwauth.to_str()?)?;
                let answer = prompt.respond(&context)?.to_header_string();
                request
                    .headers_mut()
                    .insert(AUTHORIZATION, HeaderValue::from_str(&answer)?);
                self.print(&mut ctx, &mut response, &mut request)?;
                Ok(self.next(&mut ctx, request)?)
            }
            _ => Ok(response),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parsing() {
        let expected = vec![
            ("user:", ("user", None)),
            ("user:password", ("user", Some("password"))),
            ("user:pass:with:colons", ("user", Some("pass:with:colons"))),
            (":", ("", None)),
        ];
        for (input, output) in expected {
            let (user, pass) = parse_auth(input, "").unwrap();
            assert_eq!(output, (user.as_str(), pass.as_deref()));
        }
    }
}
