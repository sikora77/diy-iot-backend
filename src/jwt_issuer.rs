use std::collections::HashMap;
use std::sync::Arc;

use chrono::{Duration, Utc};
use oxide_auth::primitives::generator;
use oxide_auth::primitives::grant::Grant;
use oxide_auth::primitives::issuer::{IssuedToken, Issuer};
use oxide_auth::primitives::issuer::{RefreshedToken, TokenType};
use oxide_auth::primitives::prelude::{RandomGenerator, TagGrant, TokenMap};

struct Token {
	/// Back link to the access token.
	access: Arc<str>,

	/// Link to a refresh token for this grant, if it exists.
	refresh: Option<Arc<str>>,

	/// The grant that was originally granted.
	grant: Grant,
}

impl Token {
	fn from_access(access: Arc<str>, grant: Grant) -> Self {
		Token {
			access,
			refresh: None,
			grant,
		}
	}

	fn from_refresh(access: Arc<str>, refresh: Arc<str>, grant: Grant) -> Self {
		Token {
			access,
			refresh: Some(refresh),
			grant,
		}
	}
}

pub struct JwtIssuer {
	duration: Option<Duration>,
	generator: RandomGenerator,
	usage: u64,
	access: HashMap<Arc<str>, Arc<Token>>,
	refresh: HashMap<Arc<str>, Arc<Token>>,
}

impl JwtIssuer {
	fn access_token(&mut self, grant: &Grant) -> String {
		return "Hewwo".to_string();
	}
	pub fn new(generator: RandomGenerator) -> Self {
		Self {
			duration: None,
			generator: generator,
			usage: 0,
			access: HashMap::new(),
			refresh: HashMap::new(),
		}
	}
	fn set_duration(&self, grant: &mut Grant) {
		if let Some(duration) = &self.duration {
			grant.until = Utc::now() + *duration;
		}
	}
}
impl Issuer for JwtIssuer {
	fn issue(&mut self, mut grant: Grant) -> Result<IssuedToken, ()> {
		self.set_duration(&mut grant);
		let (access, refresh) = {
			let access = self.access_token(&grant);
			let refresh = self.generator.tag(self.usage.wrapping_add(1), &grant)?;
			debug_assert!(
				access.len() > 0,
				"An empty access token was generated, this is horribly insecure."
			);
			debug_assert!(
				refresh.len() > 0,
				"An empty refresh token was generated, this is horribly insecure."
			);
			(access, refresh)
		};

		let next_usage = self.usage.wrapping_add(1);
		let access_key: Arc<str> = Arc::from(access.clone());
		let refresh_key: Arc<str> = Arc::from(refresh.clone());
		let until = grant.until;
		let token = Token::from_refresh(access_key.clone(), refresh_key.clone(), grant);
		let token = Arc::new(token);
		self.access.insert(access_key, token.clone());
		self.refresh.insert(refresh_key, token);
		self.usage = next_usage;
		Ok(IssuedToken {
			token: access,
			refresh: Some(refresh),
			until: until,
			token_type: oxide_auth::primitives::issuer::TokenType::Bearer,
		})
	}
	fn refresh(&mut self, refresh: &str, mut grant: Grant) -> Result<RefreshedToken, ()> {
		// Remove the old token.
		let (refresh_key, mut token) = self
			.refresh
			.remove_entry(refresh)
			// Should only be called on valid refresh tokens.
			.ok_or(())?;

		assert!(Arc::ptr_eq(token.refresh.as_ref().unwrap(), &refresh_key));
		self.set_duration(&mut grant);
		let until = grant.until;

		let new_access = self.access_token(&grant);

		// let tag = tag.wrapping_add(1);
		let tag = self.usage;

		let new_refresh = self.generator.tag(tag, &grant)?;

		let new_access_key: Arc<str> = Arc::from(new_access.clone());
		let new_refresh_key: Arc<str> = Arc::from(new_refresh.clone());

		if let Some(atoken) = self.access.remove(&token.access) {
			assert!(Arc::ptr_eq(&token, &atoken));
		}

		{
			// Should now be the only `Arc` pointing to this.
			let mut_token = Arc::get_mut(&mut token).unwrap_or_else(|| {
				unreachable!("Grant data was only shared with access and refresh")
			});
			// Remove the old access token, insert the new.
			mut_token.access = new_access_key.clone();
			mut_token.refresh = Some(new_refresh_key.clone());
			mut_token.grant = grant;
		}

		self.access.insert(new_access_key, token.clone());
		self.refresh.insert(new_refresh_key, token);

		self.usage = tag.wrapping_add(1);
		Ok(RefreshedToken {
			token: new_access,
			refresh: Some(new_refresh),
			until,
			token_type: TokenType::Bearer,
		})
	}
	fn recover_refresh<'a>(&'a self, token: &'a str) -> Result<Option<Grant>, ()> {
		Ok(self.refresh.get(token).map(|token| token.grant.clone()))
	}
	fn recover_token<'a>(&'a self, token: &'a str) -> Result<Option<Grant>, ()> {
		Ok(self.access.get(token).map(|token| token.grant.clone()))
	}
}
