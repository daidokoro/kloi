use allocative::Allocative;
use derive_more::Display;
use handlebars::Handlebars;
use starlark::starlark_simple_value;
use starlark::values::{NoSerialize, ProvidesStaticType, StarlarkValue};
use starlark_derive::starlark_value;
use std::collections::HashMap;
// use std::process::Command;
use log;

#[derive(Debug, Clone, derive_more::Display, Allocative, NoSerialize, ProvidesStaticType)]
#[allocative(skip)]
pub struct JSONValues(serde_json::Value);

#[starlark_value(type = "JSONValues", UnpackValue, StarlarkTypeRepr)]
impl<'v> StarlarkValue<'v> for JSONValues {}

#[derive(Debug, Display, ProvidesStaticType, NoSerialize, Allocative, Clone)]
#[display("Stack")]
pub struct Stack {
    // pub source: String,
    pub template: String,
    pub bucket: Option<String>,
    pub name: String,
    #[allocative(skip)]
    // pub values: Option<HashMap<String, serde_json::Value>>,
    pub values: Option<serde_json::Value>,
    pub depends_on: Option<Vec<String>>,
    pub parameters: Option<HashMap<String, String>>,
    pub region: Option<String>,
    pub capabilities: Option<Vec<String>>,
    pub exec: Option<Hooks>,
    pub custom_resources: Option<Vec<String>>,
    // pub macros: Option<HashMap<String, String>>,
}

#[starlark_value(type = "stack")]
impl<'v> StarlarkValue<'v> for Stack {}

starlark_simple_value!(Stack);

impl Stack {
    // self.is_dependency_of(s); -> bool
    pub fn is_dependency_of(&self, s: &Stack) -> bool {
        // if s.depends_on is None, return false
        if s.depends_on.is_none() {
            return false;
        }

        for dep in s.depends_on.clone().unwrap().iter() {
            if self.name == *dep {
                return true;
            }
        }
        false
    }

    // applies template values from stacks.values to generate template.
    // if not values are present, the same template is returned unmodified
    pub fn generate_template(&self) -> Result<String, String> {
        log::debug!(
            "[{}] generating template, with values: {:?}",
            self.name,
            self.values
        );
        let reg = Handlebars::new();
        if let Some(values) = self.values.clone() {
            log::debug!("rendering template with json values: {:?}", values);
            return reg.render_template(&self.template, &values).map_err(|e| {
                format!(
                    "failed to render template with json values {:?}: [{}]",
                    self.values, e,
                )
            });
        };

        Ok(self.template.to_string())
    }
}

#[derive(Debug, Display, ProvidesStaticType, NoSerialize, Allocative, Clone)]
#[display("Hook")]
pub struct Hook {
    pub name: String,
    pub run: String,
    pub on_complete: Option<bool>,
}

#[starlark_value(type = "job")]
impl<'v> StarlarkValue<'v> for Hook {}

starlark_simple_value!(Hook);

#[derive(Debug, Display, ProvidesStaticType, NoSerialize, Allocative, Clone)]
#[display("Exec")]
pub struct Hooks {
    pub on_create: Option<Vec<Hook>>,
    pub on_update: Option<Vec<Hook>>,
    pub on_delete: Option<Vec<Hook>>,
    pub on_status: Option<Vec<Hook>>,
}

#[starlark_value(type = "exec")]
impl<'v> StarlarkValue<'v> for Hooks {}

starlark_simple_value!(Hooks);
