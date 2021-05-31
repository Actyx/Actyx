use parity_multiaddr::{Multiaddr, Protocol};
use serde_json::Value;
use std::str::FromStr;
use valico::json_schema::{
    errors,
    keywords::format::FormatBuilders,
    schema, scope,
    validators::{BoxedValidator, ValidationState, Validator},
};

pub struct MultiaddrValidator {
    with_peer_id: bool,
}

impl MultiaddrValidator {
    fn new(with_peer_id: bool) -> Self {
        Self { with_peer_id }
    }

    fn validate_multiaddr(&self, val: &Value) -> Result<(), String> {
        let string = if let Some(s) = val.as_str() {
            s
        } else {
            return Err("The value must be a string".into());
        };
        let mut multiaddr = match Multiaddr::from_str(string) {
            Ok(addr) => addr,
            Err(err) => return Err(format!("Malformed multiaddr. {}", err)),
        };
        match multiaddr.pop() {
            Some(Protocol::P2p(_)) if self.with_peer_id => Ok(()),
            Some(Protocol::P2p(_)) if !self.with_peer_id => Err("Expected multiaddr without peer id.".into()),
            Some(_) if self.with_peer_id => Err("Expected multiaddr with peer id.".into()),
            Some(_) if !self.with_peer_id => Ok(()),
            None => Err("Empty multiaddr.".into()),
            _ => unreachable!(),
        }
    }
}

impl Validator for MultiaddrValidator {
    fn validate(&self, val: &Value, path: &str, _scope: &scope::Scope) -> ValidationState {
        match self.validate_multiaddr(val) {
            Ok(()) => ValidationState::new(),
            Err(err) => ValidationState {
                errors: vec![Box::new(errors::Format {
                    path: path.to_string(),
                    detail: err,
                })],
                missing: vec![],
                replacement: None,
            },
        }
    }
}

pub fn extra_formats(formats: &mut FormatBuilders) {
    let multiaddr_builder = |with_peer_id: bool| {
        Box::new(move |_def: &Value, _ctx: &schema::WalkContext| {
            Ok(Some(Box::new(MultiaddrValidator::new(with_peer_id)) as BoxedValidator))
        })
    };
    formats.insert("multiaddr-with-peer-id".to_string(), multiaddr_builder(true));
    formats.insert("multiaddr-without-peer-id".to_string(), multiaddr_builder(false));
}
