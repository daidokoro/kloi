// use crate::cli::sources::*;
use crate::config;
use crate::stacks;

use starlark::collections::SmallMap;
use starlark::environment::{GlobalsBuilder, Module};
use starlark::eval::Evaluator;
use starlark::syntax::{AstModule, Dialect};
use starlark::values::ValueLike;

use starlark::values::{list, none::NoneType, ProvidesStaticType, Value};
use starlark_derive::starlark_module;
use std::cell::RefCell;
use std::collections::HashMap;
use std::thread;

// Source trait used to read source strings such as
// filepaths, http endpoints, s3 (Todo), etc...
pub trait Source {
    fn read(&self) -> Result<String, String>;
}

impl Source for String {
    fn read(&self) -> Result<String, String> {
        if self.starts_with("http://") {
            log::debug!("reading config via http: {}", self);
            let url = self.clone();
            // threading since function will be called within an async runtime
            return thread::spawn(move || {
                let resp = reqwest::blocking::get(url).map_err(|e| format!("{:?}", e))?;

                if !resp.status().is_success() {
                    return Err(format!("failed to execute http.get: {:?}", resp.status()));
                }

                let content = resp
                    .text()
                    .map_err(|e| format!("failed to execute http.get: {:?}", e))?;

                Ok(content)
            })
            .join()
            .map_err(|e| format!("{:?}", e))?;
        }

        log::debug!("reading config from file: {}", self);
        std::fs::read_to_string(self).map_err(|e| e.to_string())
    }
}

pub struct Config {
    pub stacks: Vec<stacks::Stack>,
}

impl From<config::ConfigLoader> for Config {
    fn from(c: config::ConfigLoader) -> Self {
        Config {
            stacks: c.stacks.into_inner(),
        }
    }
}

#[derive(Debug, ProvidesStaticType, Default, Clone)]
struct ConfigLoader {
    pub stacks: RefCell<Vec<stacks::Stack>>,
}

impl ConfigLoader {
    fn add(&self, s: stacks::Stack) {
        self.stacks.borrow_mut().push(s);
    }
}

// impl starlark::UnpackValue<'_> for serde_json::Value {

// }

#[starlark_module]
pub fn starlark_stacks_module(builder: &mut GlobalsBuilder) {
    fn new(
        name: String,
        template: String,
        region: String,
        bucket: Option<String>,

        // depends_on: Option<Vec<String>>,
        // depends_on: Option<list::ListOf<String>>,
        values: Option<Value>,
        parameters: Option<SmallMap<String, String>>,
        capabilities: Option<list::ListOf<String>>,
        custom_resources: Option<list::ListOf<String>>,
        // hook: Option<Value>

        // json_values: serde_json::Value,
    ) -> anyhow::Result<stacks::Stack> {
        let mut stack = stacks::Stack {
            name: name,
            template: template,
            bucket: bucket,
            values: None,
            parameters: None,
            capabilities: None,
            region: Some(region),
            exec: None,
            depends_on: None,
            custom_resources: None,
        };

        if let Some(capabilities) = capabilities {
            let caps: Vec<String> = Some(capabilities)
                .unwrap()
                .to_vec()
                .iter()
                .map(|s| s.clone())
                .collect();
            stack.capabilities = Some(caps);
        }

        // if let Some(depends_on) = depends_on {
        //     let depends = Some(depends_on)
        //         .unwrap()
        //         .to_vec()
        //         .iter()
        //         .map(|s| s.clone())
        //         .collect();
        //     stack.depends_on = Some(depends);
        // }

        if let Some(parameters) = parameters {
            let mut params: HashMap<String, String> = HashMap::new();
            for (k, v) in parameters {
                params.insert(k.to_string(), v.to_string());
            }

            stack.parameters = Some(params);
        }

        if let Some(vals) = values {
            let value_str = serde_json::to_string(&vals)?;
            let values: serde_json::Value = serde_json::from_str(value_str.as_str())?;
            stack.values = Some(values);
        }

        if let Some(custom_resources) = custom_resources {
            let crs: Vec<String> = Some(custom_resources)
                .unwrap() // TODO: handle unwrap
                .to_vec()
                .iter()
                .map(|s| s.clone())
                .collect();
            stack.custom_resources = Some(crs);
        }

        // if let Some(exec)
        Ok(stack)
    }

    fn add(x: Value, eval: &mut Evaluator) -> anyhow::Result<NoneType> {
        let c = eval
            .extra
            .ok_or_else(|| anyhow::Error::msg("failed to add stack to config: evaluation failed"))?
            .downcast_ref::<ConfigLoader>()
            .ok_or_else(|| {
                anyhow::Error::msg("failed to add stack to config: unable to cast ConfigLoader")
            })?;

        let v = x
            .downcast_ref::<stacks::Stack>()
            .unwrap()
            .clone()
            .to_owned();
        c.add(v);

        Ok(NoneType)
    }
}

#[starlark_module]
fn os_functions(builder: &mut GlobalsBuilder) {
    // file loads a file from the filesystem
    fn open(path: String) -> anyhow::Result<String> {
        let content = std::fs::read_to_string(path.clone())
            .map_err(|e| anyhow::Error::msg(format!("failed to read config file: {:?}", e)))?;
        Ok(content)
    }

    // env loads an environment variable
    fn env(var: String) -> anyhow::Result<String> {
        let value = std::env::var(var.clone())
            .map_err(|e| anyhow::Error::msg(format!("failed to read env var: {:?}", e)))?;
        Ok(value)
    }

    // cmd executes a command
    fn cmd(cmd: String) -> anyhow::Result<String> {
        let output = std::process::Command::new("sh")
            .args(vec!["-c", cmd.as_str()])
            .output()
            .map_err(|e| {
                anyhow::Error::msg(format!(
                    "failed to execute command [{}]: [{:?}]",
                    cmd.as_str(),
                    e
                ))
            })?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

#[starlark_module]
fn http_functions(builder: &mut GlobalsBuilder) {
    // get - performs a simple  HTTP get request
    fn get(url: String, headers: Option<SmallMap<String, String>>) -> anyhow::Result<String> {
        thread::spawn(move || {
            let resp = reqwest::blocking::get(url.as_str())
                .map_err(|e| anyhow::Error::msg(format!("failed to execute http.get: {:?}", e)))?;

            if !resp.status().is_success() {
                return Err(anyhow::Error::msg(format!(
                    "failed to execute http.get: {:?}",
                    resp.status()
                )));
            }

            let content = resp
                .text()
                .map_err(|e| anyhow::Error::msg(format!("failed to execute http.get: {:?}", e)))?;

            Ok(content)
        })
        .join()
        .map_err(|e| anyhow::Error::msg(format!("{:?}", e)))?
    }

    // post - performs a simple HTTP post request
    fn post(url: String, body: String) -> anyhow::Result<String> {
        let content = reqwest::blocking::Client::new()
            .post(url.as_str())
            .body(body)
            .send()
            .map_err(|e| anyhow::Error::msg(format!("failed to read config file: {:?}", e)))?
            .text()
            .map_err(|e| anyhow::Error::msg(format!("failed to read config file: {:?}", e)))?;
        Ok(content)
    }
}

// load_config_from_file loads a config from a file and validates it
pub fn load_config_from_file(src: String) -> Result<Config, String> {
    let content = src.read()?;
    let ast = AstModule::parse(&src, content, &Dialect::Standard).map_err(|e| e.to_string())?;
    // We build our globals adding some functions we wrote
    let globals = GlobalsBuilder::new()
        .with_struct("stacks", starlark_stacks_module)
        .with_struct("os", os_functions)
        .with_struct("http", http_functions)
        .build();

    let module = Module::new();
    // let store = Store::default();
    let config = ConfigLoader {
        stacks: RefCell::new(Vec::new()),
    };

    let mut eval = Evaluator::new(&module);

    // We add a reference to our store
    eval.extra = Some(&config);
    eval.eval_module(ast, &globals).map_err(|e| e.to_string())?;

    Ok(Config::from(config.clone()))
}
