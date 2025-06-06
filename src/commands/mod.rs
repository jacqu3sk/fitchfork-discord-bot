use std::process::Command;
use serenity::model::prelude::application_command::ApplicationCommandInteraction;
use serenity::prelude::Context;

pub async fn uptime(ctx: &Context, command: &ApplicationCommandInteraction) {
    let output = Command::new("uptime")
        .output()
        .unwrap_or_else(|_| panic!("Failed to run uptime"));

    let result = String::from_utf8_lossy(&output.stdout).trim().to_string();

    let _ = command.create_interaction_response(&ctx.http, |res| {
        res.interaction_response_data(|msg| msg.content(format!("`{}`", result)))
    }).await;
}

pub async fn restart_service(ctx: &Context, command: &ApplicationCommandInteraction) {
    if let Some(option) = command.data.options.get(0) {
        let service = option.value.as_ref().unwrap().as_str().unwrap();

        let output = Command::new("systemctl")
            .arg("restart")
            .arg(service)
            .output();

        match output {
            Ok(out) => {
                if out.status.success() {
                    let _ = command.create_interaction_response(&ctx.http, |res| {
                        res.interaction_response_data(|msg| msg.content(format!("✅ Restarted `{}` successfully.", service)))
                    }).await;
                } else {
                    let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
                    let _ = command.create_interaction_response(&ctx.http, |res| {
                        res.interaction_response_data(|msg| msg.content(format!("❌ Failed to restart `{}`:\n```{}```", service, err)))
                    }).await;
                }
            }
            Err(e) => {
                let _ = command.create_interaction_response(&ctx.http, |res| {
                    res.interaction_response_data(|msg| msg.content(format!("❌ Error running command: {}", e)))
                }).await;
            }
        }
    }
}


macro_rules! shell_command {
    ($ctx:expr, $cmd:expr, $args:expr, $label:expr, $interaction:expr) => {{
        let output = Command::new($cmd)
            .args($args)
            .output();

        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
                let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
                let content = if out.status.success() {
                    format!("✅ **{}** executed successfully:\n```{}```", $label, stdout)
                } else {
                    format!("❌ **{}** failed:\n```{}```", $label, stderr)
                };
                let _ = $interaction.create_interaction_response(&$ctx.http, |res| {
                    res.interaction_response_data(|msg| msg.content(content))
                }).await;
            }
            Err(err) => {
                let _ = $interaction.create_interaction_response(&$ctx.http, |res| {
                    res.interaction_response_data(|msg| msg.content(format!("❌ Error: {}", err)))
                }).await;
            }
        }
    }};
}

pub async fn ff_clean(ctx: &Context, command: &ApplicationCommandInteraction) {
    shell_command!(ctx, "bash", &["-c", "cd ~/fitch-fork/backend && source ~/.cargo/env && cargo make clean"], "Clean", command);
}
pub async fn ff_fresh(ctx: &Context, command: &ApplicationCommandInteraction) {
    shell_command!(ctx, "bash", &["-c", "cd ~/fitch-fork/backend && source ~/.cargo/env && cargo make fresh"], "Fresh", command);
}
pub async fn ff_migrate(ctx: &Context, command: &ApplicationCommandInteraction) {
    shell_command!(ctx, "bash", &["-c", "cd ~/fitch-fork/backend && source ~/.cargo/env && cargo make migrate"], "Migrate", command);
}
pub async fn ff_restart_api(ctx: &Context, command: &ApplicationCommandInteraction) {
    shell_command!(ctx, "bash", &["~/scripts/restart-api.sh"], "Restart API", command);
}
pub async fn ff_start_api(ctx: &Context, command: &ApplicationCommandInteraction) {
    shell_command!(ctx, "bash", &["~/scripts/start-api.sh"], "Start API", command);
}
pub async fn ff_stop_api(ctx: &Context, command: &ApplicationCommandInteraction) {
    shell_command!(ctx, "bash", &["~/scripts/stop-api.sh"], "Stop API", command);
}
pub async fn ff_tail_logs(ctx: &Context, command: &ApplicationCommandInteraction) {
    shell_command!(ctx, "bash", &["-c", "tail -n 50 ~/logs/fitchfork.log"], "Tail Logs", command);
}
pub async fn ff_reboot(ctx: &Context, command: &ApplicationCommandInteraction) {
    shell_command!(ctx, "sudo", &["reboot"], "Reboot Server", command);
}