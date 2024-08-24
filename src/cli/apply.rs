use aws_config::{self, BehaviorVersion};
use aws_sdk_cloudformation::types::Capability;

use aws_types::region::Region;
use aws_types::SdkConfig;
use clap::ArgMatches;
use clap::{arg, Command};
use colored::Colorize;
use log;
use std::env;

use crate::config;
use crate::utils;
use utils::exec_jobs;
use utils::stack_request_result_handle;

use aws_smithy_types::body::SdkBody;
use aws_smithy_types::byte_stream::ByteStream;

use crate::stacks;
use md5;

const ABOUT: &str = r#"deploy or udpates stacks based on cloudformation template,
if a stack already exists, it will be updated.
"#;

pub fn command() -> Command {
    Command::new("apply")
        .about(ABOUT.truecolor(125, 174, 189).to_string())
        .alias("a")
        .arg(arg!([stack]))
        .arg(arg!(-c --config <FILE> "path to config file"))
}

pub async fn handle(matches: &ArgMatches) -> Result<(), String> {
    let mut config_path = env::var("KLOI_CONFIG").ok();

    // if config is not set by env, check if it is set by cli
    if let None = config_path {
        log::debug!("config path is not set by env, KLOI_CONFIG, check CLI -c/--config");
        config_path = Some(matches
            .get_one::<String>("config")
            .ok_or_else(|| "config file required, please supply using -c/--config or set the KLOI_CONFIG env var".to_string())?.to_string());
    };

    // load config and create client
    // note: unwrap is fine here, since we've already checked if config is set above
    let conf = config::load_config_from_file(config_path.unwrap())?;

    let mut execution_stacks = match matches.get_one::<String>("stack") {
        Some(c) => {
            if let None = conf.stacks.iter().find(|s| &s.name == c) {
                Err(format!("stack [{}] not found", c))?;
            };
            vec![conf.stacks.iter().find(|s| &s.name == c).unwrap()]
        }
        None => conf.stacks.iter().collect(),
    };

    execution_stacks.sort_by(|a, b| {
        if a.is_dependency_of(b) {
            return std::cmp::Ordering::Less;
        }

        if b.is_dependency_of(a) {
            return std::cmp::Ordering::Greater;
        }

        std::cmp::Ordering::Equal
    });

    // iterate stacks
    for stack in execution_stacks.iter() {
        // // create client per stack
        let region = stack.region.clone().unwrap_or("eu-west-1".to_string());
        let sdk_config = aws_config::defaults(BehaviorVersion::latest())
            .region(Region::new(region))
            .load()
            .await;
        let client = aws_sdk_cloudformation::Client::new(&sdk_config);

        // convert parameters to vec of Parameter
        let mut params = Vec::new();
        if let Some(p) = &stack.parameters {
            p.iter().for_each(|(k, v)| {
                let param = aws_sdk_cloudformation::types::Parameter::builder()
                    .parameter_value(v)
                    .parameter_key(k)
                    .build();
                params.push(param);
            });
        };

        // get capabilities
        let capabilities = Some(
            stack
                .capabilities
                .clone()
                .unwrap_or(vec![])
                .iter()
                .map(|c| Capability::from(c.as_str()))
                .collect(),
        );

        // run update if stack exists
        let exists = utils::stack_exists(&client, &stack.name).await;
        if let Ok(_) = exists {
            // stack exists, update
            // execute on_update hooks
            exec_jobs!(on_update, &stack, stack.name.clone(), false);
            update_stack(&client, &stack, sdk_config, capabilities, params).await?;
            exec_jobs!(on_update, &stack, stack.name.clone(), true);
            return Ok(());
        }

        // execute on_apply hook
        exec_jobs!(on_create, &stack, stack.name.clone(), false);
        create_stack(&client, &stack, sdk_config, capabilities, params).await?;
        exec_jobs!(on_create, &stack, stack.name.clone(), true);
    }

    Ok(())
}

// update_stack updates a stack
pub async fn update_stack(
    client: &aws_sdk_cloudformation::Client,
    s: &stacks::Stack,
    sdk_config: SdkConfig,
    // stack_name: &str,
    // template: &str,
    capabilities: Option<Vec<Capability>>,
    params: Vec<aws_sdk_cloudformation::types::Parameter>,
) -> Result<(), String> {
    log::debug!("update_stack function called for stack: {}", s.name);
    // load template
    let template = s.generate_template()?;

    let mut req = client
        .update_stack()
        .stack_name(&s.name)
        .set_parameters(Some(params))
        .set_capabilities(capabilities);

    // check if template is more than 52,000 bytes
    req = if template.as_bytes().len() > 51200 {
        let bucket = s.bucket.as_ref().ok_or_else(|| {
            format!(
                "[{}] error: no bucket defined for large template (>51200 bytes)",
                s.name
            )
        })?;
        let key = format!("kloi-{}", &s.template.md5());
        let url = s3upload(sdk_config, bucket.clone(), key, template.to_string()).await?;

        req.template_url(&url)
    } else {
        req.template_body(&template)
    };

    let res = req.send().await;

    stack_request_result_handle!(res, s.name, "update stack");

    // wait for stack
    utils::stackprogress(&client, &s.name, s.custom_resources.clone(), s.region.clone().unwrap(), utils::WaitEvent::Update).await
    // utils::wait_for_stack_v2(&client, &s.name, utils::WaitEvent::Update).await
}

// create_stack creates a stack
pub async fn create_stack(
    client: &aws_sdk_cloudformation::Client,
    s: &stacks::Stack,
    sdk_config: SdkConfig,
    // stack_name: &str,
    // template: &str,
    capabilities: Option<Vec<Capability>>,
    params: Vec<aws_sdk_cloudformation::types::Parameter>,
) -> Result<(), String> {
    // load template
    log::debug!("create_stack function called for stack: {}", s.name);
    let template = s.generate_template()?;

    let mut req = client
        .create_stack()
        .stack_name(&s.name)
        .set_parameters(Some(params))
        .set_capabilities(capabilities);

    // check if template is more than 52,000 bytes
    req = if template.as_bytes().len() > 51200 {
        let bucket = s.bucket.as_ref().ok_or_else(|| {
            format!(
                "[{}] error: no bucket defined for large template (>51200 bytes)",
                s.name
            )
        })?;
        let key = format!("kloi-{}", &s.template.md5());
        let url = s3upload(sdk_config, bucket.clone(), key, template.to_string()).await?;

        req.template_url(&url)
    } else {
        req.template_body(&template)
    };

    let res = req.send().await;

    stack_request_result_handle!(res, s.name, "create stack");

    // wait for stack
    utils::stackprogress(&client, &s.name, s.custom_resources.clone(), s.region.clone().unwrap(), utils::WaitEvent::Create).await
    // utils::wait_for_stack_v2(&client, &s.name, utils::WaitEvent::Create).await
}

async fn s3upload(
    sdk_config: SdkConfig,
    bucket: String,
    key: String,
    data: String,
) -> Result<String, String> {
    log::debug!("uploading template to s3: s3://{}/{}", bucket, key);
    let client = aws_sdk_s3::Client::new(&sdk_config);
    let buffer = ByteStream::new(SdkBody::from(data));

    client
        .put_object()
        .bucket(bucket.clone())
        .key(key.clone())
        .body(buffer)
        .send()
        .await
        .map_err(|e| format!("failed uploading file to bucket - {}", e))?;

    //    Ok(format!("https://{}.s3.amazonaws.com/{}", bucket, key))
    Ok(format!("https://s3.amazonaws.com/{}/{}", bucket, key))
}

// Md5Sum is a trait for generating md5 hash of a string
trait Md5Sum {
    fn md5(&self) -> String;
}

// implement Md5Sum for String
impl Md5Sum for String {
    fn md5(&self) -> String {
        let digest = md5::compute(self.as_bytes());
        format!("{:x}", digest)
    }
}
