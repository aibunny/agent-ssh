use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    process::ExitCode,
};

use agent_ssh_broker::{AuditedOutcome, Broker, RunRequest, describe_invocation};
use agent_ssh_common::load_config;
use clap::{Args, Parser, Subcommand};

// ── CLI definition ────────────────────────────────────────────────────────────

#[derive(Debug, Parser)]
#[command(
    name = "agent-ssh",
    version,
    about = "Security-first SSH broker: run named commands on named servers without exposing credentials"
)]
struct Cli {
    /// Path to the TOML configuration file.
    /// Falls back to $AGENT_SSH_CONFIG, then agent-ssh.toml in the current directory.
    #[arg(long, global = true)]
    config: Option<PathBuf>,
    /// Label recorded in every audit event to identify the caller.
    #[arg(long, global = true, default_value = "cli")]
    actor: String,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Configuration management
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
    /// Server listing
    Hosts {
        #[command(subcommand)]
        command: HostsCommand,
    },
    /// Profile listing
    Profiles {
        #[command(subcommand)]
        command: ProfilesCommand,
    },
    /// Plan a run request (shows what would execute, does not connect)
    Run(RunArgs),
    /// Plan and execute a run request, capturing all output
    Exec(ExecArgs),
    /// Create a starter agent-ssh.toml in the current directory
    Init(InitArgs),
}

#[derive(Debug, Subcommand)]
enum ConfigCommand {
    /// Validate the configuration file and exit
    Validate,
}

#[derive(Debug, Subcommand)]
enum HostsCommand {
    /// List all configured server aliases
    List,
}

#[derive(Debug, Subcommand)]
enum ProfilesCommand {
    /// List profiles allowed for a given server
    List(ProfileListArgs),
}

#[derive(Debug, Args)]
struct ProfileListArgs {
    #[arg(long)]
    server: String,
}

#[derive(Debug, Args)]
struct RunArgs {
    #[arg(long)]
    server: String,
    #[arg(long)]
    profile: String,
    /// Named arguments in key=value form, repeated for each argument.
    #[arg(long = "arg")]
    args: Vec<String>,
    /// Approval reference (ticket ID, change record, etc.) required for
    /// servers and profiles that have requires_approval = true.
    #[arg(long)]
    approval: Option<String>,
}

#[derive(Debug, Args)]
struct ExecArgs {
    #[arg(long)]
    server: String,
    #[arg(long)]
    profile: String,
    /// Named arguments in key=value form, repeated for each argument.
    #[arg(long = "arg")]
    args: Vec<String>,
    /// Approval reference required for requires_approval servers/profiles.
    #[arg(long)]
    approval: Option<String>,
    /// Show the exact SSH command that would be run, then exit without executing.
    #[arg(long)]
    dry_run: bool,
}

#[derive(Debug, Args)]
struct InitArgs {
    /// Path to write the starter config (default: agent-ssh.toml).
    #[arg(long, default_value = "agent-ssh.toml")]
    output: PathBuf,
    /// Overwrite the file if it already exists.
    #[arg(long)]
    force: bool,
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<ExitCode, String> {
    let cli = Cli::parse();

    // `init` runs before any config is loaded.
    if let Command::Init(ref args) = cli.command {
        return handle_init(args).map(|()| ExitCode::SUCCESS);
    }

    let config_path = resolve_config_path(cli.config)?;
    let config = load_config(&config_path).map_err(|e| e.to_string())?;
    let secret_env = build_secret_env(&config_path)?;
    let broker =
        Broker::from_config_with_secret_env(config, secret_env).map_err(|e| e.to_string())?;
    let logger = broker.audit_logger();

    match cli.command {
        Command::Init(_) => unreachable!(),

        Command::Config {
            command: ConfigCommand::Validate,
        } => {
            logger
                .append(&broker.config_validated_event(&cli.actor))
                .map_err(|e| e.to_string())?;
            println!("configuration is valid: {}", config_path.display());
            println!("audit log: {}", broker.audit_log_path().display());
        }

        Command::Hosts {
            command: HostsCommand::List,
        } => {
            let hosts = record_outcome(&logger, broker.list_hosts(&cli.actor))?;
            for host in hosts {
                println!(
                    "{}\tenvironment={}\tuser={}\trequires_approval={}",
                    host.alias, host.environment, host.user, host.requires_approval
                );
            }
        }

        Command::Profiles {
            command: ProfilesCommand::List(args),
        } => {
            let profiles = record_outcome(&logger, broker.list_profiles(&cli.actor, &args.server))?;
            for profile in profiles {
                println!(
                    "{}\trequires_approval={}\tdescription={}",
                    profile.name,
                    profile.requires_approval,
                    profile.description.unwrap_or_else(|| "-".to_string())
                );
            }
        }

        Command::Run(args) => {
            let named_args = parse_named_args(args.args)?;
            let plan = record_outcome(
                &logger,
                broker.plan_run(RunRequest {
                    actor: cli.actor.clone(),
                    server_alias: args.server,
                    profile: args.profile,
                    args: named_args,
                    approval_reference: args.approval,
                }),
            )?;

            println!("server:           {}", plan.server_alias);
            println!(
                "target:           {}@{}:{}",
                plan.user, plan.host, plan.port
            );
            println!("environment:      {}", plan.environment);
            println!("profile:          {}", plan.profile);
            println!("auth_method:      {}", plan.auth_method_label());
            println!("signer:           {}", plan.signer);
            println!("requires_approval:{}", plan.requires_approval);
            println!("approval_provided:{}", plan.approval_provided);
            println!("rendered_command: {}", plan.rendered_command);
            println!("execution_mode:   {:?}", plan.execution_mode);
            println!("audit_log:        {}", broker.audit_log_path().display());
            println!();
            println!("(Use `agent-ssh exec` to plan and run this command.)");
        }

        Command::Exec(args) => {
            let named_args = parse_named_args(args.args)?;

            let request = RunRequest {
                actor: cli.actor.clone(),
                server_alias: args.server,
                profile: args.profile,
                args: named_args,
                approval_reference: args.approval,
            };

            // If --dry-run: plan only, print the SSH command, exit without executing.
            if args.dry_run {
                let plan = record_outcome(&logger, broker.plan_run(request))?;
                println!("dry-run: would execute the following SSH command:");
                println!();
                println!("  {}", describe_invocation(&plan));
                println!();
                println!("target:  {}@{}:{}", plan.user, plan.host, plan.port);
                println!("command: {}", plan.rendered_command);
                return Ok(ExitCode::SUCCESS);
            }

            // Live execution: plan then SSH.
            let (plan_outcome, exec_outcome) = broker.run(request);

            // Record both audit events; abort on write error.
            logger
                .append(&plan_outcome.audit_event)
                .map_err(|e| e.to_string())?;
            logger
                .append(&exec_outcome.audit_event)
                .map_err(|e| e.to_string())?;

            // Surface planning errors.
            let plan = plan_outcome.result.map_err(|e| e.to_string())?;

            // Surface execution errors (ssh not found, spawn failure, etc.)
            let output = exec_outcome.result.map_err(|e| e.to_string())?;

            // Print the output the agent / human always needs to see.
            print_exec_output(&plan, &output);

            // Non-zero exit codes propagate back to the shell.
            if output.exit_code != 0 {
                return Ok(ExitCode::FAILURE);
            }
        }
    }

    Ok(ExitCode::SUCCESS)
}

// ── Output formatting ─────────────────────────────────────────────────────────

fn print_exec_output(plan: &agent_ssh_broker::RunPlan, output: &agent_ssh_broker::CommandOutput) {
    // Header — always present so agents can parse the block reliably.
    eprintln!(
        "--- agent-ssh exec: {}  {}@{}:{}  profile={} ---",
        plan.server_alias, plan.user, plan.host, plan.port, plan.profile
    );

    // Stdout — always print, even if empty.
    if output.stdout.is_empty() {
        eprintln!("[stdout: empty]");
    } else {
        print!("{}", output.stdout);
        // Ensure a trailing newline if the command didn't emit one.
        if !output.stdout.ends_with('\n') {
            println!();
        }
    }

    // Stderr — only print if non-empty.
    if !output.stderr.is_empty() {
        eprintln!("--- stderr ---");
        eprint!("{}", output.stderr);
        if !output.stderr.ends_with('\n') {
            eprintln!();
        }
    }

    eprintln!("--- exit {} ---", output.exit_code);
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn record_outcome<T>(
    logger: &agent_ssh_broker::AuditLogger,
    outcome: AuditedOutcome<T>,
) -> Result<T, String> {
    logger
        .append(&outcome.audit_event)
        .map_err(|e| e.to_string())?;
    outcome.result.map_err(|e| e.to_string())
}

fn parse_named_args(raw_args: Vec<String>) -> Result<BTreeMap<String, String>, String> {
    let mut parsed = BTreeMap::new();

    for raw_arg in raw_args {
        let Some((name, value)) = raw_arg.split_once('=') else {
            return Err(format!(
                "invalid --arg '{raw_arg}'; expected the form key=value"
            ));
        };

        if name.is_empty() {
            return Err("argument names must not be empty".to_string());
        }

        if parsed.insert(name.to_string(), value.to_string()).is_some() {
            return Err(format!("duplicate --arg key '{name}'"));
        }
    }

    Ok(parsed)
}

fn resolve_config_path(cli_path: Option<PathBuf>) -> Result<PathBuf, String> {
    if let Some(path) = cli_path {
        return Ok(path);
    }

    if let Ok(path) = env::var("AGENT_SSH_CONFIG") {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            return Ok(PathBuf::from(trimmed));
        }
    }

    let default_path = PathBuf::from("agent-ssh.toml");
    if default_path.is_file() {
        return Ok(default_path);
    }

    Err("no configuration file found\n\
         Options:\n\
         \x20 --config <path>        pass the path explicitly\n\
         \x20 $AGENT_SSH_CONFIG      set the environment variable\n\
         \x20 agent-ssh.toml         place a config in the current directory\n\
         \n\
         Run `agent-ssh init` to create a starter configuration."
        .to_string())
}

fn build_secret_env(config_path: &Path) -> Result<BTreeMap<String, String>, String> {
    let mut env_map = env::vars().collect::<BTreeMap<_, _>>();
    let dotenv_path = resolve_dotenv_path(config_path);
    if !dotenv_path.is_file() {
        return Ok(env_map);
    }

    let source = fs::read_to_string(&dotenv_path).map_err(|error| {
        format!(
            "failed to read dotenv file at {}: {error}",
            dotenv_path.display()
        )
    })?;
    let dotenv_values = parse_dotenv(&source)?;

    for (name, value) in dotenv_values {
        env_map.entry(name).or_insert(value);
    }

    Ok(env_map)
}

fn resolve_dotenv_path(config_path: &Path) -> PathBuf {
    if let Ok(path) = env::var("AGENT_SSH_ENV_FILE") {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }

    config_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(".env")
}

fn parse_dotenv(source: &str) -> Result<BTreeMap<String, String>, String> {
    let mut values = BTreeMap::new();

    for (index, raw_line) in source.lines().enumerate() {
        let line_number = index + 1;
        let trimmed = raw_line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let line = trimmed.strip_prefix("export ").unwrap_or(trimmed);
        let Some((name, raw_value)) = line.split_once('=') else {
            return Err(format!(
                "invalid .env line {line_number}: expected KEY=VALUE"
            ));
        };
        let name = name.trim();
        if !is_valid_env_var_name(name) {
            return Err(format!(
                "invalid .env line {line_number}: '{name}' is not a valid environment variable name"
            ));
        }

        let value = normalize_dotenv_value(raw_value.trim())
            .map_err(|reason| format!("invalid .env line {line_number}: {reason}"))?;
        values.insert(name.to_string(), value);
    }

    Ok(values)
}

fn normalize_dotenv_value(value: &str) -> Result<String, String> {
    if value.is_empty() {
        return Ok(String::new());
    }

    if (value.starts_with('"') && value.ends_with('"'))
        || (value.starts_with('\'') && value.ends_with('\''))
    {
        if value.len() < 2 {
            return Err("quoted values must be balanced".to_string());
        }
        return Ok(value[1..value.len() - 1].to_string());
    }

    if value.starts_with('"')
        || value.starts_with('\'')
        || value.ends_with('"')
        || value.ends_with('\'')
    {
        return Err("quoted values must start and end with the same quote".to_string());
    }

    Ok(value.to_string())
}

fn is_valid_env_var_name(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    if !matches!(first, 'A'..='Z' | 'a'..='z' | '_') {
        return false;
    }

    chars.all(|char| matches!(char, 'A'..='Z' | 'a'..='z' | '0'..='9' | '_'))
}

// ── init subcommand ───────────────────────────────────────────────────────────

/// Starter configuration written by `agent-ssh init`.
const STARTER_CONFIG: &str = r#"# agent-ssh.toml — generated by `agent-ssh init`
#
# Edit the values marked <CHANGE ME> then run:
#   agent-ssh config validate
#
# Quick-start:
#   1. Fill in your server host / user below.
#   2. Run:  agent-ssh config validate
#   3. Run:  agent-ssh exec --server my-server --profile disk

# ── Broker settings ───────────────────────────────────────────────────────────

[broker]
# How long (seconds) a certificate-backed SSH session stays valid.
# Keep this short; 120 s (2 min) is a safe default.
cert_ttl_seconds = 120

# All broker decisions are appended here in JSONL format.
audit_log_path = "./data/audit.jsonl"

# Name of the signer used by default (must match a [signers.*] entry below).
default_signer = "step_ca"

# ── Signers ───────────────────────────────────────────────────────────────────
# A signer issues short-lived SSH certificates.  "step-ca" (smallstep) is the
# most common choice.

[signers.step_ca]
kind       = "step-ca"
ca_url     = "https://ca.internal.example"   # <CHANGE ME>
provisioner = "agent-ssh"
subject    = "agent-ssh-broker"

# ── Servers ───────────────────────────────────────────────────────────────────
# Each entry is a named SSH target.  Callers use the alias (e.g. "my-server");
# the actual host / port / credentials are never exposed to them.

# Example: certificate-authenticated server (default auth_method).
[servers.my-server]                           # <CHANGE ME: alias>
host        = "10.0.1.10"                    # <CHANGE ME: IP or hostname>
port        = 22
user        = "deploy"                        # <CHANGE ME: SSH username>
environment = "staging"                       # <CHANGE ME: staging | production | …>
allowed_profiles = ["logs", "disk"]

# Example: approval-required production server.
# [servers.prod-web]
# host        = "10.0.10.21"                 # <CHANGE ME>
# port        = 22
# user        = "deploy"                     # <CHANGE ME>
# environment = "production"
# allowed_profiles = ["logs"]
# requires_approval = true                   # caller must pass --approval <ref>
#
# Example: legacy password compatibility server. The password itself must NOT
# live in TOML or .env. Instead, store an opaque reference in a sibling .env:
#   AGENT_SSH_LEGACY_WEB_PASSWORD_REF=os_keychain:agent-ssh:legacy-web
#
# [servers.legacy-web]
# host        = "10.0.20.5"
# port        = 22
# user        = "deploy"
# environment = "migration"
# allowed_profiles = ["logs"]
# auth_method = "legacy_password"
# password_secret_ref_env_var = "AGENT_SSH_LEGACY_WEB_PASSWORD_REF"
# legacy_password_acknowledged = true
# fail2ban_allowlist_confirmed = true

# ── Profiles ──────────────────────────────────────────────────────────────────
# Profiles define the exact commands callers are allowed to run.
# Templates use {{placeholder}} tokens filled in at request time.
# Shell metacharacters (|  ;  >  `  $  &) are FORBIDDEN in templates.

[profiles.logs]
description = "Tail systemd service logs"
template    = "journalctl -u {{service}} --since {{since}} --no-pager"

[profiles.disk]
description = "Show disk usage"
template    = "df -h"

# [profiles.ps]
# description = "List running processes"
# template    = "ps aux"

# Operational note:
# agent-ssh uses publickey-only non-interactive SSH and will not fall back to
# passwords or keyboard-interactive prompts. If your infrastructure uses
# fail2ban and routes broker traffic through fixed egress IPs, allowlist those
# IPs/CIDRs in fail2ban's ignoreip setting on the remote hosts.
"#;

fn handle_init(args: &InitArgs) -> Result<(), String> {
    if args.output.exists() && !args.force {
        return Err(format!(
            "{} already exists — use --force to overwrite",
            args.output.display()
        ));
    }

    fs::write(&args.output, STARTER_CONFIG)
        .map_err(|e| format!("failed to write {}: {e}", args.output.display()))?;

    println!("created: {}", args.output.display());
    println!();
    println!("next steps:");
    println!(
        "  1. Edit {} — fill in your server host, user, and environment",
        args.output.display()
    );
    println!(
        "  2. agent-ssh config validate --config {}",
        args.output.display()
    );
    println!(
        "  3. agent-ssh exec --config {} --server my-server --profile disk",
        args.output.display()
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::{build_secret_env, parse_dotenv};

    #[test]
    fn parse_dotenv_accepts_export_and_quotes() {
        let parsed = parse_dotenv(
            r#"
# comment
export AGENT_SSH_PASSWORD_REF="os_keychain:agent-ssh:legacy-web"
PLAIN_REF='os_keychain:agent-ssh:legacy-db'
"#,
        )
        .expect(".env should parse");

        assert_eq!(
            parsed.get("AGENT_SSH_PASSWORD_REF").map(String::as_str),
            Some("os_keychain:agent-ssh:legacy-web")
        );
        assert_eq!(
            parsed.get("PLAIN_REF").map(String::as_str),
            Some("os_keychain:agent-ssh:legacy-db")
        );
    }

    #[test]
    fn parse_dotenv_rejects_invalid_env_var_names() {
        let error = parse_dotenv("BAD-NAME=value").expect_err("bad env var name must fail");
        assert!(error.contains("not a valid environment variable name"));
    }

    #[test]
    fn build_secret_env_reads_sibling_dotenv() {
        let tempdir = tempdir().expect("tempdir");
        let config_path = tempdir.path().join("agent-ssh.toml");
        let dotenv_path = tempdir.path().join(".env");
        fs::write(&config_path, "# config placeholder").expect("config");
        fs::write(
            &dotenv_path,
            "AGENT_SSH_LEGACY_DB_PASSWORD_REF=os_keychain:agent-ssh:legacy-db\n",
        )
        .expect("dotenv");

        let env_map = build_secret_env(&config_path).expect("secret env");

        assert_eq!(
            env_map
                .get("AGENT_SSH_LEGACY_DB_PASSWORD_REF")
                .map(String::as_str),
            Some("os_keychain:agent-ssh:legacy-db")
        );
    }
}
