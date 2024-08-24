use aws_sdk_cloudformation::types::{ResourceStatus, StackEvent};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
// use isatty::stdout_isatty;
use aws_config::{self, BehaviorVersion};
use aws_sdk_cloudformation::Client;
use regex::Regex;
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::thread;
use tokio::time::{sleep, Duration};
use chrono::{Utc, TimeZone};

// enum for wait events
#[derive(Clone)]
pub enum WaitEvent {
    Create,
    Update,
    Delete,
}

impl WaitEvent {
    fn to_string(&self) -> String {
        match self {
            WaitEvent::Create => "create".to_string(),
            WaitEvent::Update => "update".to_string(),
            WaitEvent::Delete => "delete".to_string(),
        }
    }
}

fn format_status(status: Option<&ResourceStatus>) -> String {
    match status.clone().unwrap() {
        ResourceStatus::CreateComplete => "create complete".green(),
        ResourceStatus::CreateFailed => "create failed".red(),
        ResourceStatus::CreateInProgress => "create in progress".yellow(),
        ResourceStatus::DeleteComplete => "delete complete".green(),
        ResourceStatus::DeleteFailed => "delete failed".red(),
        ResourceStatus::DeleteInProgress => "delete in progress".yellow(),
        ResourceStatus::DeleteSkipped => "delete skipped".yellow(),
        ResourceStatus::ImportComplete => "import complete".green(),
        ResourceStatus::ImportFailed => "import failed".red(),
        ResourceStatus::ImportInProgress => "import in progress".yellow(),
        ResourceStatus::ImportRollbackComplete => "import rollback complete".green(),
        ResourceStatus::ImportRollbackFailed => "import rollback failed".red(),
        ResourceStatus::ImportRollbackInProgress => "import rollback in progress".yellow(),
        ResourceStatus::RollbackComplete => "rollback complete".red(),
        ResourceStatus::RollbackFailed => "rollback failed".red(),
        ResourceStatus::RollbackInProgress => "rollback in progress".red(),
        ResourceStatus::UpdateComplete => "update complete".green(),
        ResourceStatus::UpdateInProgress => "update in progress".yellow(),
        ResourceStatus::UpdateRollbackComplete => "update rollback complete".green(),
        ResourceStatus::UpdateRollbackInProgress => {
            "update rollback complete cleanup in progress".yellow()
        }
        ResourceStatus::UpdateRollbackFailed => "update rollback failed".red(),
        _ => {
            if let Some(s) = status {
                s.to_string().blue()
            } else {
                "unknown".blue()
            }
        }
    }
    .to_string()
}

// use colored::*;
// use futures::StreamExt;

pub async fn stackprogress(
    client: &Client,
    stack_name: &str,
    custom_respirces: Option<Vec<String>>,
    region: String,
    wait_event: WaitEvent,
) -> Result<(), String> {
    if std::env::var("KLOI_LOG").unwrap_or("".to_string()) == "debug" {
        return wait_for_stack(client, stack_name, wait_event).await;
    }

    // let (tx, mut rx) = mpsc::channel(100);

    let pb = ProgressBar::new(0);
    let spinner_style =
        ProgressStyle::with_template("{prefix:.bold.dim} {spinner:.green}{wide_msg}\n")
            .unwrap()
            .tick_strings(&[
                "[ ●    ]",
                "[  ●   ]",
                "[   ●  ]",
                "[    ● ]",
                "[     ●]",
                "[    ● ]",
                "[   ●  ]",
                "[  ●   ]",
                "[ ●    ]",
                "[●     ]",
                "",
            ]);
    pb.set_style(spinner_style.clone());
    pb.enable_steady_tick(Duration::from_millis(100));
    pb.set_prefix(format!("[{}] [{}]", "info".green(), stack_name.cyan()));

    let break_re = Regex::new(r"complete|failed").unwrap();
    let mut status = String::new();
    // let mut events: Vec<String> = Vec::new();
    let mut events = HashMap::<String, Option<String>>::new();
    // let previous_status = String::new();

    while !break_re.is_match(status.to_lowercase().as_str()) {
        let status_res = client.describe_stacks().stack_name(stack_name).send().await;

        match status_res {
            Ok(r) => {
                r.stacks().iter().for_each(|s| {
                    status = s
                        .stack_status()
                        .unwrap() // TODO: handle unwrap
                        .to_string();

                    
                    let stk = ResourceStatus::from(status.as_str());

          
                    pb.set_prefix(format!(
                        "[{}] [{}] {}",
                        "info".green(),
                        stack_name.cyan(),
                        format_status(Some(&stk))
                    ));
                });
            }
            Err(e) => {
                let err = format!(
                    "error describing stack for wait: {:?}",
                    e.into_service_error().meta().message()
                );

                if !err.contains("does not exist") {
                    Err(err.clone())?;
                }

                // assume deleted if stack does not exist
                if let WaitEvent::Delete = wait_event {
                    pb.set_prefix(format!("[{}] [{}]", "info".green(), stack_name.cyan()));
                    pb.finish_with_message(format!("{}", "does not exist".yellow()));
                    return Ok(());
                }

                Err(err)?;
            }
        }

        let stack_events_res = client
            .describe_stack_events()
            .stack_name(stack_name)
            .send()
            .await;

        let failed_events: Vec<&StackEvent> = match stack_events_res.as_ref() {
            Ok(r) => r
                .stack_events()
                .iter()
                .filter(|e| {
                    return format_status(e.resource_status()).contains("failed");
                })
                .collect(),
            Err(_) => {
                pb.finish_with_message(format!("[{}] error", stack_name.cyan()));
                return Err(format!("[{}] error", stack_name.cyan()));
            }
        };

        for e in failed_events.iter() {
            let event_status = format_status(e.resource_status());
            let resource = e.resource_type().unwrap();
            let reason = e.resource_status_reason().unwrap_or("executing");

            let msg = format!(
                "\n---\n{}\n{}\nstatus: {}\nreason: {}",
                e.logical_resource_id().unwrap(),
                resource.truecolor(45, 51, 59),
                event_status.truecolor(45, 51, 59),
                reason.red()
            );
            if events.contains_key(&msg) {
                continue;
            }

            events.insert(msg, None);
        }

        let e: Vec<String> = events.keys().map(|v| v.to_string()).collect();
        pb.set_message(e.join("\n"));

        sleep(Duration::from_secs(2)).await;
    }

    // get custom resources
    if let Some(crs) = custom_respirces {
        for cr in crs.iter() {

            let physical_id = client
                .describe_stack_resources()
                .stack_name(stack_name)
                .logical_resource_id(cr.clone())
                .send()
                .await
                .map_err(|e| {
                    e.into_service_error()
                        .meta()
                        .message()
                        .unwrap_or("unknown error")
                        .to_string()
                })?
                .stack_resources()
                .iter()
                .next()
                .unwrap() // TODO: handle unwrap
                .physical_resource_id()
                .unwrap()
                .to_string(); // TODO: handle unwrap

            let logs = get_cloudwatch_logs(physical_id, region.clone()).await?;
            // pb.finish_with_message(format!("[{}] {}", cr.cyan(), logs));
            events.insert(format!("\n---\n[{}]\n{}---\n", cr.bold(), logs).truecolor(96, 96, 96).to_string(), None);
        }
    }

    let e: Vec<String> = events.keys().map(|v| v.to_string()).collect();
    // pb.finish_with_message(format!("{}", e.join("\n")));
    pb.finish();
    println!("{}", e.join("\n"));

    Ok(())
}

// wait_for_stack_completion waits for a stack to reach a failed or complete state
pub async fn wait_for_stack(
    client: &aws_sdk_cloudformation::Client,
    stack_name: &str,
    wait_event: WaitEvent,
) -> Result<(), String> {
    let mut do_break = false;
    let break_re = Regex::new(r"complete|failed").unwrap();

    // hash of string, none type
    let mut seen = std::collections::HashMap::<String, Option<String>>::new();
    let mut current_status = String::new();
    let mut previous_status = String::new();
    let stack_status_res = client.describe_stacks().stack_name(stack_name).send().await;
    match stack_status_res {
        Ok(r) => {
            r.stacks().iter().for_each(|s| {
                current_status = s
                    .stack_status()
                    .unwrap() // TODO: handle unwrap
                    .to_string();
            });
        }
        Err(e) => {
            Err(format!(
                "error describing stack for wait: {}",
                e.into_service_error()
            ))?;
        }
    }

    'outer: loop {
        let stack_events_res = client
            .describe_stack_events()
            .stack_name(stack_name)
            .send()
            .await;

        if current_status != previous_status {
            let s = ResourceStatus::from(current_status.as_str());
            // log::info!("[{}] {}: {}",
            //     stack_name.cyan(),
            //     "status".truecolor(96, 96, 96),
            //     format_status(Some(&s)));
            log::info!(
                "{}",
                format!("[{}] {}", stack_name, &s).truecolor(96, 96, 96)
            );

            previous_status = current_status.clone();
        }

        match stack_events_res {
            Ok(r) => {
                r.stack_events().iter().for_each(|e| {
                    let status = format_status(e.resource_status());
                    let reason = match e.resource_status_reason() {
                        Some(r) => r,
                        None => "executing",
                    };

                    let msg = format!(
                        "[{0: <1}] {1: <35} {2: <30} {3: <10}",
                        // "[{:<20}] {:<20} {:<35} {}",
                        e.stack_name().unwrap().cyan(),
                        status,
                        e.resource_type().unwrap().bright_purple(),
                        // e.stack_id().unwrap(),
                        reason
                    );

                    if !seen.contains_key(&msg) && status.contains(wait_event.to_string().as_str())
                    {
                        log::info!("{}", msg);
                        seen.insert(msg, None);
                    }
                });
            }
            Err(e) => {
                let err = format!(
                    "error describing stack for wait: {:?}",
                    e.into_service_error().to_string()
                );

                if !err.contains("does not exist") {
                    Err(err.clone())?;
                }

                // assume deleted if stack does not exist
                if let WaitEvent::Delete = wait_event {
                    log::info!(
                        "[{}] {} - {}",
                        stack_name.cyan(),
                        "delete complete".green(),
                        "stack does not exist"
                    );
                    return Ok(());
                }

                Err(err)?;
            }
        }

        // check stack statu
        let stack_res = client.describe_stacks().stack_name(stack_name).send().await;

        // check stack status and break if complete or failed
        match stack_res {
            Ok(r) => {
                r.stacks().iter().for_each(|s| {
                    current_status = s.stack_status().unwrap().to_string();
                    if break_re.is_match(&current_status.to_lowercase().as_str()) {
                        do_break = true;
                    }
                });
            }
            Err(e) => {
                Err(format!(
                    "error describing stack for wait: {}",
                    e.into_service_error()
                ))?;
            }
        }

        // break if stack is complete or failed
        if do_break {
            let s = ResourceStatus::from(current_status.as_str());
            log::info!(
                "{}",
                format!("[{}] {}", stack_name, &s).truecolor(96, 96, 96)
            );
            break 'outer;
        }

        // sleep for 2 seconds
        sleep(Duration::from_secs(2)).await;
    }

    Ok(())
}

// stack_exists checks if a stack exists
// returns an error if the stack does not exist
pub async fn stack_exists(
    client: &aws_sdk_cloudformation::Client,
    stack_name: &str,
) -> Result<(), String> {
    let stack_res = client.describe_stacks().stack_name(stack_name).send().await;

    match stack_res {
        Ok(r) => {
            if r.stacks().len() > 0 {
                Ok(())
            } else {
                // technically this should never happen
                Err(format!("stack {} does not exist", stack_name))
            }
        }
        Err(e) => Err(format!(
            "error describing stack: {}",
            e.into_service_error()
        )),
    }
}

// stack_exec used to execute subprocess commands
// for stack events
pub fn stack_exec(
    stack_name: String,
    process_name: String,
    command_str: String,
) -> Result<(), String> {
    let command = "sh";
    let args = ["-c", command_str.as_str()];
    let mut process = Command::new(command)
        .args(&args)
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| {
            format!(
                "failed to execute exec command {} for {}: {}",
                process_name, stack_name, e
            )
        })?;

    let stack = stack_name.clone();
    let exec_name = process_name.clone();

    if let Some(stdout) = process.stdout.take() {
        let reader = BufReader::new(stdout);
        let reader_handle = thread::spawn(move || {
            log::info!(
                "[{}] executing -> {}:\n---",
                stack_name.cyan(),
                process_name.green()
            );
            reader
                .lines()
                .filter_map(|line| line.ok())
                .for_each(|line| {
                    println!("{}", line.truecolor(96, 96, 96));
                });
            println!("---")
        });

        // Wait for the reader thread to finish.å
        if let Err(e) = reader_handle.join() {
            Err(format!(
                "failed to join the reader thread for exec command {} for {}: {:?}",
                exec_name, stack, e
            ))?
        }

        // Wait for the command to finish.
        match process.wait() {
            Ok(status) => {
                if status.success() {
                    return Ok(());
                }
                return Err(format!(
                    "failed to execute exec command {} for {}: {}",
                    exec_name, stack, status
                ));
            }
            Err(e) => {
                return Err(format!(
                    "failed waiting for exec command {} for {}: {}",
                    exec_name, stack, e
                ));
            }
        }
    }

    Ok(())
}

// get_cloudwatch_logs
async fn get_cloudwatch_logs(lambda_id: String, region: String) -> Result<String, String> {
    let loggroup = format!("/aws/lambda/{}", lambda_id);
    // log::info!("fetching logs for loggroup: {} in {}", loggroup, region);
    // sleep 10 seconds for logs to be available
    sleep(Duration::from_secs(5)).await;
    let sdk_config = aws_config::defaults(BehaviorVersion::latest())
        .region(aws_types::region::Region::new(region))
        .load()
        .await;

    let client = aws_sdk_cloudwatchlogs::Client::new(&sdk_config);

    let streams = client
        .describe_log_streams()
        .log_group_name(&loggroup)
        .send()
        .await
        .map_err(|e| {
            e.into_service_error()
                .meta()
                .message()
                .unwrap_or("unknown error")
                .to_string()
        })?;

    if streams.log_streams().len() == 0 {
        return Ok("no logs found".to_string());
    };

    let mut lines: Vec<String>;

    loop {
        let stream = streams.log_streams().iter().next().unwrap();
        let events = client
            .get_log_events()
            .log_group_name(&loggroup)
            .log_stream_name(stream.log_stream_name().unwrap())
            .send()
            .await
            .map_err(|e| {
                e.into_service_error()
                    .meta()
                    .message()
                    .unwrap_or("unknown error")
                    .to_string()
            })?;
    
        // ts - convert epoch to rfc3339
        let ts = |ts: String| -> String {
            let epoch = ts.parse::<i64>().expect("Failed to parse epoch string");
            let dt = Utc.timestamp_opt(epoch, 0).unwrap();
            dt.to_rfc3339()
        };
    
        lines = events
            .events()
            .iter()
            .map(|e| {
                format!(
                    "[{}] {}",
                    ts(e.timestamp().unwrap().to_string()),
                    e.message().unwrap_or("no message")
                )
                .truecolor(96, 96, 96)
                .to_string()
            })
            .collect::<Vec<String>>();

        if lines.join("").contains("END RequestId") {
            break;
        }

        sleep(Duration::from_secs(4)).await;
    }



    Ok(format!(
        "{}:\n{}",
        loggroup.cyan().bold(),
        lines.join("")
    ))
}

/// handles the result of a stack requests
/// by parsing the errors and returning a structured error string (if any)
/// The macro expects:
/// - stack request result
/// - stack name (String)
/// - request type (String)
///  # Example:
/// ```
/// stack_request_result_handle!(stack_request_response, stack.name, "create stack");
/// ````
#[macro_export]
macro_rules! stack_request_result_handle {
    ($res:expr, $stack_name:expr, $req:expr) => {
        use aws_sdk_cloudformation::error::ProvideErrorMetadata;
        match $res {
            Ok(_) => {
                log::info!(
                    "[{}] {} {}",
                    $stack_name.cyan(),
                    $req.green(),
                    "request executed".green()
                );
            }
            Err(e) => {
                let code = e.code().unwrap_or("no error code");
                let message = e.message().unwrap_or("unknown error");
                Err(format!(
                    "[{}] error occurred during {} request: {} - {}",
                    $stack_name, $req, code, message
                ))?;
            }
        }
    };
}

/// exec_jobs macro executes exec jobs for a given stack
/// the macro expects an event, stack, name and post flag.
/// The _post_ flag indicates if the job is being run after the
/// event has been executed
///
/// # Examples
///
/// __run jobs before create__
/// ```
/// exec_jobs!(on_create, $stack, "my_stack", false);
/// ```
/// __run jobs after create__
/// ```
/// exec_jobs!(on_create, $stack, "my_stack", true);
/// ```
#[macro_export]
macro_rules! exec_jobs {
    ($event:ident, $stack:expr, $name:expr, $post:expr) => {
        if let Some(exec) = $stack.exec.as_ref() {
            let empty_jobs_vec: Vec<crate::stacks::Hook> = vec![];
            let jobs = &exec.$event.as_ref().unwrap_or(&empty_jobs_vec);
            for job in jobs.iter() {
                if let Some(true) = job.on_complete {
                    if !$post {
                        continue;
                    }
                }

                if job.on_complete.unwrap_or(false) != $post {
                    continue;
                }

                if let Err(e) = utils::stack_exec($name, job.name.clone(), job.run.clone()) {
                    return Err(e);
                }
            }
        }
    };
}

// make macro public
pub(crate) use exec_jobs;
pub(crate) use stack_request_result_handle;
